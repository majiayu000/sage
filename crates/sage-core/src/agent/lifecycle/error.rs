//! Lifecycle error types

use crate::agent::AgentState;
use crate::error::{SageError, UnifiedError};
use std::fmt;

use super::phase::LifecyclePhase;

/// Result type for lifecycle operations
pub type LifecycleResult<T> = Result<T, LifecycleError>;

/// Errors that can occur during lifecycle operations
///
/// Implements the `UnifiedError` trait for consistent error handling across crates.
#[derive(Debug, Clone)]
pub enum LifecycleError {
    /// Initialization failed
    InitFailed {
        message: String,
        context: Option<String>,
    },
    /// Hook execution failed
    HookFailed {
        hook: LifecyclePhase,
        message: String,
        context: Option<String>,
    },
    /// State transition not allowed
    InvalidTransition {
        from: AgentState,
        to: AgentState,
        context: Option<String>,
    },
    /// Shutdown failed
    ShutdownFailed {
        message: String,
        context: Option<String>,
    },
    /// Hook aborted the operation
    Aborted {
        phase: LifecyclePhase,
        reason: String,
        context: Option<String>,
    },
    /// Wrapped sage error
    Internal {
        message: String,
        context: Option<String>,
    },
}

impl LifecycleError {
    /// Create a new InitFailed error
    pub fn init_failed(message: impl Into<String>) -> Self {
        Self::InitFailed {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new HookFailed error
    pub fn hook_failed(hook: LifecyclePhase, message: impl Into<String>) -> Self {
        Self::HookFailed {
            hook,
            message: message.into(),
            context: None,
        }
    }

    /// Create a new InvalidTransition error
    pub fn invalid_transition(from: AgentState, to: AgentState) -> Self {
        Self::InvalidTransition {
            from,
            to,
            context: None,
        }
    }

    /// Create a new ShutdownFailed error
    pub fn shutdown_failed(message: impl Into<String>) -> Self {
        Self::ShutdownFailed {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Aborted error
    pub fn aborted(phase: LifecyclePhase, reason: impl Into<String>) -> Self {
        Self::Aborted {
            phase,
            reason: reason.into(),
            context: None,
        }
    }

    /// Create a new Internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            context: None,
        }
    }

    /// Add context to any lifecycle error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        let ctx = Some(context.into());
        match &mut self {
            Self::InitFailed { context: c, .. } => *c = ctx,
            Self::HookFailed { context: c, .. } => *c = ctx,
            Self::InvalidTransition { context: c, .. } => *c = ctx,
            Self::ShutdownFailed { context: c, .. } => *c = ctx,
            Self::Aborted { context: c, .. } => *c = ctx,
            Self::Internal { context: c, .. } => *c = ctx,
        }
        self
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitFailed { message, .. } => write!(f, "Initialization failed: {}", message),
            Self::HookFailed { hook, message, .. } => {
                write!(f, "Hook {} failed: {}", hook, message)
            }
            Self::InvalidTransition { from, to, .. } => {
                write!(f, "Invalid state transition from {} to {}", from, to)
            }
            Self::ShutdownFailed { message, .. } => write!(f, "Shutdown failed: {}", message),
            Self::Aborted { phase, reason, .. } => {
                write!(f, "Aborted at {}: {}", phase, reason)
            }
            Self::Internal { message, .. } => write!(f, "Internal error: {}", message),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl UnifiedError for LifecycleError {
    fn error_code(&self) -> &str {
        match self {
            Self::InitFailed { .. } => "LIFECYCLE_INIT_FAILED",
            Self::HookFailed { .. } => "LIFECYCLE_HOOK_FAILED",
            Self::InvalidTransition { .. } => "LIFECYCLE_INVALID_TRANSITION",
            Self::ShutdownFailed { .. } => "LIFECYCLE_SHUTDOWN_FAILED",
            Self::Aborted { .. } => "LIFECYCLE_ABORTED",
            Self::Internal { .. } => "LIFECYCLE_INTERNAL",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::InitFailed { message, .. } => message,
            Self::HookFailed { message, .. } => message,
            Self::InvalidTransition { .. } => "Invalid state transition",
            Self::ShutdownFailed { message, .. } => message,
            Self::Aborted { reason, .. } => reason,
            Self::Internal { message, .. } => message,
        }
    }

    fn context(&self) -> Option<&str> {
        match self {
            Self::InitFailed { context, .. } => context.as_deref(),
            Self::HookFailed { context, .. } => context.as_deref(),
            Self::InvalidTransition { context, .. } => context.as_deref(),
            Self::ShutdownFailed { context, .. } => context.as_deref(),
            Self::Aborted { context, .. } => context.as_deref(),
            Self::Internal { context, .. } => context.as_deref(),
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(self, Self::HookFailed { .. } | Self::ShutdownFailed { .. })
    }
}

impl From<SageError> for LifecycleError {
    fn from(err: SageError) -> Self {
        Self::internal(err.to_string())
    }
}
