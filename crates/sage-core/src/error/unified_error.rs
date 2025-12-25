//! UnifiedError trait implementation for SageError

use super::types::{SageError, UnifiedError};

/// Implement UnifiedError trait for SageError
impl UnifiedError for SageError {
    fn error_code(&self) -> &str {
        match self {
            Self::Config { .. } => "SAGE_CONFIG",
            Self::Llm { .. } => "SAGE_LLM",
            Self::Tool { .. } => "SAGE_TOOL",
            Self::Agent { .. } => "SAGE_AGENT",
            Self::Cache { .. } => "SAGE_CACHE",
            Self::Io { .. } => "SAGE_IO",
            Self::Json { .. } => "SAGE_JSON",
            Self::Http { .. } => "SAGE_HTTP",
            Self::InvalidInput { .. } => "SAGE_INVALID_INPUT",
            Self::Timeout { .. } => "SAGE_TIMEOUT",
            Self::Cancelled => "SAGE_CANCELLED",
            Self::Storage { .. } => "SAGE_STORAGE",
            Self::NotFound { .. } => "SAGE_NOT_FOUND",
            Self::Other { .. } => "SAGE_OTHER",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Config { message, .. } => message,
            Self::Llm { message, .. } => message,
            Self::Tool { message, .. } => message,
            Self::Agent { message, .. } => message,
            Self::Cache { message, .. } => message,
            Self::Io { message, .. } => message,
            Self::Json { message, .. } => message,
            Self::Http { message, .. } => message,
            Self::InvalidInput { message, .. } => message,
            Self::Timeout { .. } => "Task execution timeout",
            Self::Cancelled => "Task was cancelled",
            Self::Storage { message, .. } => message,
            Self::NotFound { message, .. } => message,
            Self::Other { message, .. } => message,
        }
    }

    fn context(&self) -> Option<&str> {
        match self {
            Self::Config { context, .. } => context.as_deref(),
            Self::Llm { context, .. } => context.as_deref(),
            Self::Tool { context, .. } => context.as_deref(),
            Self::Agent { context, .. } => context.as_deref(),
            Self::Cache { context, .. } => context.as_deref(),
            Self::Io { context, .. } => context.as_deref(),
            Self::Json { context, .. } => context.as_deref(),
            Self::Http { context, .. } => context.as_deref(),
            Self::InvalidInput { context, .. } => context.as_deref(),
            Self::Timeout { context, .. } => context.as_deref(),
            Self::Cancelled => None,
            Self::Storage { context, .. } => context.as_deref(),
            Self::NotFound { context, .. } => context.as_deref(),
            Self::Other { context, .. } => context.as_deref(),
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Http { .. } | Self::Timeout { .. } | Self::Llm { .. }
        )
    }
}
