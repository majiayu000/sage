//! Lifecycle phase definitions

use std::fmt;

/// Lifecycle phases where hooks can be registered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecyclePhase {
    /// Agent initialization (before first task)
    Init,
    /// Before task execution starts
    TaskStart,
    /// Before each step in the execution
    StepStart,
    /// After each step completes
    StepComplete,
    /// After task execution completes (success or failure)
    TaskComplete,
    /// Agent shutdown
    Shutdown,
    /// State transition
    StateTransition,
    /// Error occurred
    Error,
}

impl fmt::Display for LifecyclePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => write!(f, "init"),
            Self::TaskStart => write!(f, "task_start"),
            Self::StepStart => write!(f, "step_start"),
            Self::StepComplete => write!(f, "step_complete"),
            Self::TaskComplete => write!(f, "task_complete"),
            Self::Shutdown => write!(f, "shutdown"),
            Self::StateTransition => write!(f, "state_transition"),
            Self::Error => write!(f, "error"),
        }
    }
}
