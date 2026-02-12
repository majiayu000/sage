//! Configuration types for tool orchestration

use crate::recovery::supervisor::SupervisionPolicy;
use std::path::PathBuf;
use std::time::Duration;

/// Context for tool execution, providing session and environment info
#[derive(Clone)]
pub struct ToolExecutionContext {
    /// Session ID for hook input
    pub session_id: String,
    /// Working directory for hook execution
    pub working_dir: PathBuf,
}

impl ToolExecutionContext {
    /// Create a new execution context
    pub fn new(session_id: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            session_id: session_id.into(),
            working_dir,
        }
    }
}

/// Configuration for tool execution supervision
#[derive(Debug, Clone)]
pub struct SupervisionConfig {
    /// Whether supervision is enabled
    pub enabled: bool,
    /// Supervision policy for tool failures
    pub policy: SupervisionPolicy,
}

impl Default for SupervisionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: SupervisionPolicy::Restart {
                max_restarts: 2,
                window: Duration::from_secs(60),
            },
        }
    }
}

impl SupervisionConfig {
    /// Create supervision config with no retries (for tools that shouldn't retry)
    pub fn no_retry() -> Self {
        Self {
            enabled: true,
            policy: SupervisionPolicy::Stop,
        }
    }

    /// Disable supervision entirely
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            policy: SupervisionPolicy::Stop,
        }
    }
}

/// Configuration for checkpoint behavior
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Whether checkpointing is enabled
    pub enabled: bool,
    /// Whether to auto-rollback on tool failure
    pub auto_rollback_on_failure: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_rollback_on_failure: false, // Disabled by default for safety
        }
    }
}

impl CheckpointConfig {
    /// Create config with auto-rollback enabled
    pub fn with_auto_rollback() -> Self {
        Self {
            enabled: true,
            auto_rollback_on_failure: true,
        }
    }

    /// Disable checkpointing entirely
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            auto_rollback_on_failure: false,
        }
    }
}

/// Result of the pre-execution phase
pub enum PreExecutionResult {
    /// Continue with execution
    Continue,
    /// Blocked by hook with reason
    Blocked(String),
}

impl PreExecutionResult {
    /// Check if execution should continue
    pub fn should_continue(&self) -> bool {
        matches!(self, PreExecutionResult::Continue)
    }

    /// Get block reason if blocked
    pub fn block_reason(&self) -> Option<&str> {
        match self {
            PreExecutionResult::Blocked(reason) => Some(reason),
            PreExecutionResult::Continue => None,
        }
    }
}
