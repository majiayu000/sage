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

/// Internal supervision action
#[derive(Debug)]
pub(super) enum SupervisionAction {
    Restart,
    Resume,
    Stop,
    Escalate,
}
