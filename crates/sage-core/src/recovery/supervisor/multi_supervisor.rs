//! Multi-task supervisor for managing multiple tasks

use super::super::RecoverableError;
use super::task_supervisor::TaskSupervisor;
use super::types::{SupervisionEvent, SupervisionPolicy, SupervisionResult};
use crate::error::SageError;
use std::future::Future;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

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
                            error: super::super::to_recoverable(&e),
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
