//! Single task supervisor for managing task lifecycle

use super::super::{RecoverableError, RecoveryError};
use super::types::{SupervisionAction, SupervisionPolicy, SupervisionResult};
use crate::error::SageError;
use std::future::Future;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Task supervisor for managing task lifecycle
pub struct TaskSupervisor {
    /// Task name (for logging)
    name: String,
    /// Supervision policy
    policy: SupervisionPolicy,
    /// Cancellation token
    cancel_token: CancellationToken,
    /// Restart count
    restart_count: u32,
    /// Last restart time
    last_restart: Option<Instant>,
    /// Error handler callback
    error_handler: Option<Box<dyn Fn(&RecoverableError) + Send + Sync>>,
}

impl TaskSupervisor {
    /// Create a new task supervisor
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            policy: SupervisionPolicy::default(),
            cancel_token: CancellationToken::new(),
            restart_count: 0,
            last_restart: None,
            error_handler: None,
        }
    }

    /// Set the supervision policy
    pub fn with_policy(mut self, policy: SupervisionPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set the cancellation token
    pub fn with_cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = token;
        self
    }

    /// Set an error handler callback
    pub fn on_error<F>(mut self, handler: F) -> Self
    where
        F: Fn(&RecoverableError) + Send + Sync + 'static,
    {
        self.error_handler = Some(Box::new(handler));
        self
    }

    /// Get a child cancellation token
    pub fn child_token(&self) -> CancellationToken {
        self.cancel_token.child_token()
    }

    /// Run a task with supervision (single attempt)
    ///
    /// This executes the task once and returns the supervision result.
    /// Use `run()` to execute with automatic restart handling.
    pub async fn supervise<T, F, Fut>(&mut self, task_factory: F) -> SupervisionResult
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, SageError>> + Send,
        T: Send,
    {
        if self.cancel_token.is_cancelled() {
            return SupervisionResult::Stopped {
                error: RecoverableError::permanent("Cancelled"),
            };
        }

        let result = tokio::select! {
            _ = self.cancel_token.cancelled() => {
                return SupervisionResult::Stopped {
                    error: RecoverableError::permanent("Cancelled"),
                };
            }
            result = task_factory() => result
        };

        match result {
            Ok(_) => SupervisionResult::Completed,
            Err(error) => {
                let recoverable = super::super::to_recoverable(&error);

                // Call error handler if set
                if let Some(ref handler) = self.error_handler {
                    handler(&recoverable);
                }

                match self.handle_error(&recoverable) {
                    SupervisionAction::Restart => {
                        self.restart_count += 1;
                        self.last_restart = Some(Instant::now());

                        tracing::warn!(
                            task = %self.name,
                            attempt = self.restart_count,
                            error = %recoverable.message,
                            "Restarting task after failure"
                        );

                        // Apply backoff delay
                        let delay = self.calculate_restart_delay();
                        tokio::time::sleep(delay).await;

                        SupervisionResult::Restarted {
                            attempt: self.restart_count,
                        }
                    }
                    SupervisionAction::Resume => SupervisionResult::Resumed { error: recoverable },
                    SupervisionAction::Stop => SupervisionResult::Stopped { error: recoverable },
                    SupervisionAction::Escalate => {
                        SupervisionResult::Escalated { error: recoverable }
                    }
                }
            }
        }
    }

    /// Run task continuously with supervision until completion or max restarts
    pub async fn run<T, F, Fut>(&mut self, task_factory: F) -> Result<T, RecoveryError>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: Future<Output = Result<T, SageError>> + Send,
        T: Send,
    {
        loop {
            let result = self.supervise(task_factory.clone()).await;
            match result {
                SupervisionResult::Completed => {
                    // Need to actually get the result
                    // This is a simplified implementation
                    return Err(RecoveryError::PermanentFailure {
                        message: "Task completed but result not captured".into(),
                    });
                }
                SupervisionResult::Restarted { .. } => {
                    // Continue loop to restart
                    continue;
                }
                SupervisionResult::Resumed { error } => {
                    return Err(RecoveryError::PermanentFailure {
                        message: error.message,
                    });
                }
                SupervisionResult::Stopped { error } => {
                    return Err(RecoveryError::PermanentFailure {
                        message: error.message,
                    });
                }
                SupervisionResult::Escalated { error } => {
                    return Err(RecoveryError::PermanentFailure {
                        message: format!("Escalated: {}", error.message),
                    });
                }
            }
        }
    }

    fn handle_error(&self, error: &RecoverableError) -> SupervisionAction {
        match &self.policy {
            SupervisionPolicy::Restart {
                max_restarts,
                window,
            } => {
                // Check if we've exceeded max restarts in the window
                if let Some(last_restart) = self.last_restart {
                    if last_restart.elapsed() > *window {
                        // Outside window, reset counter conceptually
                        if error.is_retryable() {
                            return SupervisionAction::Restart;
                        }
                    }
                }

                if self.restart_count < *max_restarts && error.is_retryable() {
                    SupervisionAction::Restart
                } else {
                    // Stop for both permanent errors and non-retryable errors
                    SupervisionAction::Stop
                }
            }
            SupervisionPolicy::Resume => SupervisionAction::Resume,
            SupervisionPolicy::Stop => SupervisionAction::Stop,
            SupervisionPolicy::Escalate => SupervisionAction::Escalate,
        }
    }

    fn calculate_restart_delay(&self) -> Duration {
        // Exponential backoff for restarts
        let base = Duration::from_millis(100);
        let max = Duration::from_secs(30);
        let multiplier = 2.0_f64;

        let delay_ms = base.as_millis() as f64 * multiplier.powi(self.restart_count as i32);

        Duration::from_millis(delay_ms.min(max.as_millis() as f64) as u64)
    }
}
