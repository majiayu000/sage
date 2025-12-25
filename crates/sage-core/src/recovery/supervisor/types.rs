//! Types for task supervision

use super::super::RecoverableError;
use std::time::Duration;

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
    TaskRestarted { task_name: String, attempt: u32 },
    /// Supervisor shutting down
    ShuttingDown,
}

/// Internal supervision action
#[derive(Debug)]
pub(super) enum SupervisionAction {
    Restart,
    Resume,
    Stop,
    Escalate,
}
