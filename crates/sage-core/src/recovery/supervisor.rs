//! Supervision and error isolation for tasks
//!
//! Provides supervision strategies for managing task lifecycles and failures.

use super::{ErrorClass, RecoverableError, RecoveryError};
use crate::error::SageError;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Supervision policy for task failures
#[derive(Debug, Clone)]
pub enum SupervisionPolicy {
    /// Restart the task on failure
    Restart {
        /// Maximum number of restarts
        max_restarts: u32,
        /// Time window for restart counting
        window: Duration,
    },
    /// Resume the task with error handling
    Resume,
    /// Stop the task on any failure
    Stop,
    /// Escalate the failure to the parent supervisor
    Escalate,
}

impl Default for SupervisionPolicy {
    fn default() -> Self {
        Self::Restart {
            max_restarts: 3,
            window: Duration::from_secs(60),
        }
    }
}

/// Result of supervision decision
#[derive(Debug)]
pub enum SupervisionResult {
    /// Task completed successfully
    Completed,
    /// Task restarted
    Restarted { attempt: u32 },
    /// Task resumed after error
    Resumed { error: RecoverableError },
    /// Task stopped due to failure
    Stopped { error: RecoverableError },
    /// Failure escalated to parent
    Escalated { error: RecoverableError },
}

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

    /// Run a task with supervision
    pub async fn supervise<T, F, Fut>(&mut self, task_factory: F) -> SupervisionResult
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, SageError>> + Send,
        T: Send,
    {
        loop {
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
                Ok(_) => return SupervisionResult::Completed,
                Err(error) => {
                    let recoverable = super::to_recoverable(&error);

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

                            return SupervisionResult::Restarted {
                                attempt: self.restart_count,
                            };
                        }
                        SupervisionAction::Resume => {
                            return SupervisionResult::Resumed {
                                error: recoverable,
                            };
                        }
                        SupervisionAction::Stop => {
                            return SupervisionResult::Stopped {
                                error: recoverable,
                            };
                        }
                        SupervisionAction::Escalate => {
                            return SupervisionResult::Escalated {
                                error: recoverable,
                            };
                        }
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
            SupervisionPolicy::Restart { max_restarts, window } => {
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
                } else if error.class == ErrorClass::Permanent {
                    SupervisionAction::Stop
                } else {
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
        let delay = Duration::from_millis(delay_ms.min(max.as_millis() as f64) as u64);

        delay
    }
}

#[derive(Debug)]
enum SupervisionAction {
    Restart,
    Resume,
    Stop,
    Escalate,
}

/// Supervisor for multiple tasks
pub struct Supervisor {
    /// Name of the supervisor
    #[allow(dead_code)]
    name: String,
    /// Default policy for supervised tasks
    default_policy: SupervisionPolicy,
    /// Cancellation token for all supervised tasks
    cancel_token: CancellationToken,
    /// Event channel for supervision events
    events: broadcast::Sender<SupervisionEvent>,
}

/// Events emitted by the supervisor
#[derive(Debug, Clone)]
pub enum SupervisionEvent {
    /// Task started
    TaskStarted { task_name: String },
    /// Task completed successfully
    TaskCompleted { task_name: String },
    /// Task failed
    TaskFailed {
        task_name: String,
        error: String,
        will_restart: bool,
    },
    /// Task restarted
    TaskRestarted {
        task_name: String,
        attempt: u32,
    },
    /// Supervisor shutting down
    ShuttingDown,
}

impl Supervisor {
    /// Create a new supervisor
    pub fn new(name: impl Into<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            name: name.into(),
            default_policy: SupervisionPolicy::default(),
            cancel_token: CancellationToken::new(),
            events,
        }
    }

    /// Set the default supervision policy
    pub fn with_policy(mut self, policy: SupervisionPolicy) -> Self {
        self.default_policy = policy;
        self
    }

    /// Get an event subscriber
    pub fn subscribe(&self) -> broadcast::Receiver<SupervisionEvent> {
        self.events.subscribe()
    }

    /// Create a task supervisor with the default policy
    pub fn task(&self, name: impl Into<String>) -> TaskSupervisor {
        TaskSupervisor::new(name)
            .with_policy(self.default_policy.clone())
            .with_cancel_token(self.cancel_token.child_token())
    }

    /// Spawn a supervised task (FnOnce - no restart support)
    pub fn spawn_once<F, Fut>(
        &self,
        name: impl Into<String>,
        task: F,
    ) -> JoinHandle<SupervisionResult>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), SageError>> + Send + 'static,
    {
        let task_name: String = name.into();
        let cancel_token = self.cancel_token.child_token();
        let events = self.events.clone();

        tokio::spawn(async move {
            let _ = events.send(SupervisionEvent::TaskStarted {
                task_name: task_name.clone(),
            });

            // Execute with cancellation support
            let result = tokio::select! {
                _ = cancel_token.cancelled() => {
                    SupervisionResult::Stopped {
                        error: RecoverableError::permanent("Cancelled"),
                    }
                }
                result = task() => {
                    match result {
                        Ok(()) => SupervisionResult::Completed,
                        Err(e) => SupervisionResult::Stopped {
                            error: super::to_recoverable(&e),
                        },
                    }
                }
            };

            match &result {
                SupervisionResult::Completed => {
                    let _ = events.send(SupervisionEvent::TaskCompleted {
                        task_name: task_name.clone(),
                    });
                }
                SupervisionResult::Stopped { error }
                | SupervisionResult::Escalated { error }
                | SupervisionResult::Resumed { error } => {
                    let _ = events.send(SupervisionEvent::TaskFailed {
                        task_name: task_name.clone(),
                        error: error.message.clone(),
                        will_restart: false,
                    });
                }
                SupervisionResult::Restarted { attempt } => {
                    let _ = events.send(SupervisionEvent::TaskRestarted {
                        task_name: task_name.clone(),
                        attempt: *attempt,
                    });
                }
            }

            result
        })
    }

    /// Spawn a supervised task with restart support (requires Fn)
    pub fn spawn<F, Fut>(&self, name: impl Into<String>, task: F) -> JoinHandle<SupervisionResult>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SageError>> + Send + 'static,
    {
        let task_name: String = name.into();
        let cancel_token = self.cancel_token.child_token();
        let events = self.events.clone();
        let policy = self.default_policy.clone();

        tokio::spawn(async move {
            let _ = events.send(SupervisionEvent::TaskStarted {
                task_name: task_name.clone(),
            });

            let mut supervisor = TaskSupervisor::new(task_name.clone())
                .with_policy(policy)
                .with_cancel_token(cancel_token);

            let result = supervisor.supervise(&task).await;

            match &result {
                SupervisionResult::Completed => {
                    let _ = events.send(SupervisionEvent::TaskCompleted {
                        task_name: task_name.clone(),
                    });
                }
                SupervisionResult::Stopped { error } | SupervisionResult::Escalated { error } => {
                    let _ = events.send(SupervisionEvent::TaskFailed {
                        task_name: task_name.clone(),
                        error: error.message.clone(),
                        will_restart: false,
                    });
                }
                SupervisionResult::Restarted { attempt } => {
                    let _ = events.send(SupervisionEvent::TaskRestarted {
                        task_name: task_name.clone(),
                        attempt: *attempt,
                    });
                }
                SupervisionResult::Resumed { error } => {
                    let _ = events.send(SupervisionEvent::TaskFailed {
                        task_name: task_name.clone(),
                        error: error.message.clone(),
                        will_restart: false,
                    });
                }
            }

            result
        })
    }

    /// Shutdown all supervised tasks
    pub fn shutdown(&self) {
        let _ = self.events.send(SupervisionEvent::ShuttingDown);
        self.cancel_token.cancel();
    }
}

/// Execute a function with panic recovery
pub async fn catch_panic<T, F, Fut>(task: F) -> Result<T, RecoveryError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    match std::panic::catch_unwind(AssertUnwindSafe(|| task())) {
        Ok(future) => Ok(future.await),
        Err(panic) => {
            let message = if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            Err(RecoveryError::TaskPanic { message })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_task_supervisor_success() {
        let mut supervisor = TaskSupervisor::new("test");

        let result = supervisor.supervise(|| async { Ok::<_, SageError>(42) }).await;

        assert!(matches!(result, SupervisionResult::Completed));
    }

    #[tokio::test]
    async fn test_task_supervisor_transient_failure() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let mut supervisor = TaskSupervisor::new("test").with_policy(SupervisionPolicy::Restart {
            max_restarts: 3,
            window: Duration::from_secs(60),
        });

        let result = supervisor
            .supervise(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(SageError::Http("timeout".into()))
                    } else {
                        Ok(())
                    }
                }
            })
            .await;

        // First call should result in restart
        assert!(
            matches!(result, SupervisionResult::Restarted { .. })
                || matches!(result, SupervisionResult::Completed)
        );
    }

    #[tokio::test]
    async fn test_task_supervisor_permanent_failure() {
        let mut supervisor = TaskSupervisor::new("test");

        let result = supervisor
            .supervise(|| async { Err::<(), _>(SageError::Config("invalid".into())) })
            .await;

        assert!(matches!(result, SupervisionResult::Stopped { .. }));
    }

    #[tokio::test]
    async fn test_supervisor_spawn_once() {
        let supervisor = Supervisor::new("test_supervisor");
        let mut events = supervisor.subscribe();

        let handle = supervisor.spawn_once("test_task", || async { Ok(()) });

        // Wait for completion
        let result = handle.await.unwrap();
        assert!(matches!(result, SupervisionResult::Completed));

        // Should receive start and complete events
        let event = events.recv().await.unwrap();
        assert!(matches!(event, SupervisionEvent::TaskStarted { .. }));
    }

    #[tokio::test]
    async fn test_supervisor_spawn_restartable() {
        let supervisor = Supervisor::new("test_supervisor");
        let mut events = supervisor.subscribe();

        let handle = supervisor.spawn("test_task", || async { Ok(()) });

        // Wait for completion
        let result = handle.await.unwrap();
        assert!(matches!(result, SupervisionResult::Completed));

        // Should receive start and complete events
        let event = events.recv().await.unwrap();
        assert!(matches!(event, SupervisionEvent::TaskStarted { .. }));
    }

    #[tokio::test]
    async fn test_supervisor_shutdown() {
        let supervisor = Supervisor::new("test_supervisor");
        let mut events = supervisor.subscribe();

        supervisor.shutdown();

        // Should receive shutdown event
        let event = events.recv().await.unwrap();
        assert!(matches!(event, SupervisionEvent::ShuttingDown));
    }

    #[tokio::test]
    async fn test_catch_panic() {
        let result: Result<i32, RecoveryError> = catch_panic(|| async { 42 }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
