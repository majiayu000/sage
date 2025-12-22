//! Lifecycle error types

use crate::agent::AgentState;
use crate::error::SageError;
use std::fmt;

use super::phase::LifecyclePhase;

/// Result type for lifecycle operations
pub type LifecycleResult<T> = Result<T, LifecycleError>;

/// Errors that can occur during lifecycle operations
#[derive(Debug, Clone)]
pub enum LifecycleError {
    /// Initialization failed
    InitFailed(String),
    /// Hook execution failed
    HookFailed {
        hook: LifecyclePhase,
        message: String,
    },
    /// State transition not allowed
    InvalidTransition { from: AgentState, to: AgentState },
    /// Shutdown failed
    ShutdownFailed(String),
    /// Hook aborted the operation
    Aborted {
        phase: LifecyclePhase,
        reason: String,
    },
    /// Wrapped sage error
    Internal(String),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::HookFailed { hook, message } => {
                write!(f, "Hook {} failed: {}", hook, message)
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid state transition from {} to {}", from, to)
            }
            Self::ShutdownFailed(msg) => write!(f, "Shutdown failed: {}", msg),
            Self::Aborted { phase, reason } => {
                write!(f, "Aborted at {}: {}", phase, reason)
            }
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<SageError> for LifecycleError {
    fn from(err: SageError) -> Self {
        Self::Internal(err.to_string())
    }
}
