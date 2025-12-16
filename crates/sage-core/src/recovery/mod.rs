//! Error recovery system for Sage Agent
//!
//! This module provides comprehensive error recovery capabilities:
//! - Error classification (transient vs permanent)
//! - Retry strategies with exponential backoff
//! - Circuit breaker pattern for failing dependencies
//! - Supervision and error isolation
//! - Graceful degradation

pub mod backoff;
pub mod circuit_breaker;
pub mod rate_limiter;
pub mod retry;
pub mod supervisor;

pub use backoff::{BackoffConfig, BackoffStrategy, ExponentialBackoff};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use rate_limiter::{
    RateLimitError, RateLimitGuard, RateLimiter, RateLimiterConfig, SlidingWindowRateLimiter,
};
pub use retry::{RetryConfig, RetryPolicy, RetryResult, Retryable};
pub use supervisor::{SupervisionPolicy, SupervisionResult, Supervisor, TaskSupervisor};

use std::time::Duration;
use thiserror::Error;

/// Error classification for recovery decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient errors that may succeed on retry
    Transient,
    /// Permanent errors that will not succeed on retry
    Permanent,
    /// Unknown errors - attempt limited retries
    Unknown,
}

/// Error with recovery context
#[derive(Debug, Clone)]
pub struct RecoverableError {
    /// The underlying error message
    pub message: String,
    /// Error classification
    pub class: ErrorClass,
    /// Suggested retry delay (if transient)
    pub retry_after: Option<Duration>,
    /// Error source context
    pub context: ErrorContext,
}

impl RecoverableError {
    /// Create a transient error
    pub fn transient(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Transient,
            retry_after: None,
            context: ErrorContext::default(),
        }
    }

    /// Create a transient error with retry delay
    pub fn transient_with_delay(message: impl Into<String>, delay: Duration) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Transient,
            retry_after: Some(delay),
            context: ErrorContext::default(),
        }
    }

    /// Create a permanent error
    pub fn permanent(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Permanent,
            retry_after: None,
            context: ErrorContext::default(),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Unknown,
            retry_after: None,
            context: ErrorContext::default(),
        }
    }

    /// Add context to the error
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self.class, ErrorClass::Transient | ErrorClass::Unknown)
    }
}

/// Context about where an error occurred
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// Component where the error occurred
    pub component: Option<String>,
    /// Operation that was being performed
    pub operation: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(component: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            component: Some(component.into()),
            operation: Some(operation.into()),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Recovery-specific error types
#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("Max retries exceeded: {attempts} attempts")]
    MaxRetriesExceeded { attempts: u32 },

    #[error("Circuit breaker open for {component}")]
    CircuitBreakerOpen { component: String },

    #[error("Timeout waiting for recovery after {elapsed:?}")]
    RecoveryTimeout { elapsed: Duration },

    #[error("Permanent failure: {message}")]
    PermanentFailure { message: String },

    #[error("Task panicked: {message}")]
    TaskPanic { message: String },
}

/// Classify common errors into error classes
pub fn classify_error(error: &crate::error::SageError) -> ErrorClass {
    use crate::error::SageError;

    match error {
        // Network/HTTP errors are usually transient
        SageError::Http(msg) => {
            if msg.contains("timeout")
                || msg.contains("connection refused")
                || msg.contains("connection reset")
                || msg.contains("503")
                || msg.contains("502")
                || msg.contains("504")
            {
                ErrorClass::Transient
            } else if msg.contains("401") || msg.contains("403") || msg.contains("404") {
                ErrorClass::Permanent
            } else if msg.contains("429") {
                // Rate limited - definitely transient
                ErrorClass::Transient
            } else {
                ErrorClass::Unknown
            }
        }

        // IO errors are often transient
        SageError::Io(msg) => {
            if msg.contains("permission denied") || msg.contains("not found") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Transient
            }
        }

        // LLM errors
        SageError::Llm(msg) => {
            if msg.contains("rate limit") || msg.contains("overloaded") {
                ErrorClass::Transient
            } else if msg.contains("invalid") || msg.contains("context length") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Unknown
            }
        }

        // Timeouts are transient
        SageError::Timeout { .. } => ErrorClass::Transient,

        // Cancellation is not really an error
        SageError::Cancelled => ErrorClass::Permanent,

        // Configuration and input errors are permanent
        SageError::Config(_) | SageError::InvalidInput(_) => ErrorClass::Permanent,

        // JSON errors are usually permanent (bad data)
        SageError::Json(_) => ErrorClass::Permanent,

        // Tool errors need more context
        SageError::Tool { message, .. } => {
            if message.contains("timeout") || message.contains("temporarily") {
                ErrorClass::Transient
            } else {
                ErrorClass::Unknown
            }
        }

        // Agent and cache errors
        SageError::Agent(_) | SageError::Cache(_) => ErrorClass::Unknown,

        // Storage errors are often transient
        SageError::Storage(msg) => {
            if msg.contains("permission denied") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Transient
            }
        }

        // Not found errors are permanent
        SageError::NotFound(_) => ErrorClass::Permanent,

        // Other errors
        SageError::Other(_) => ErrorClass::Unknown,
    }
}

/// Convert a SageError to a RecoverableError
pub fn to_recoverable(error: &crate::error::SageError) -> RecoverableError {
    let class = classify_error(error);
    let retry_after = extract_retry_after(error);

    RecoverableError {
        message: error.to_string(),
        class,
        retry_after,
        context: ErrorContext::default(),
    }
}

/// Extract retry-after duration from error if available
fn extract_retry_after(error: &crate::error::SageError) -> Option<Duration> {
    use crate::error::SageError;

    match error {
        SageError::Http(msg) if msg.contains("429") => {
            // Try to parse retry-after from the message
            // Format: "429: retry after 30 seconds"
            if let Some(pos) = msg.find("retry after") {
                let rest = &msg[pos + 12..];
                if let Some(seconds) = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    return Some(Duration::from_secs(seconds));
                }
            }
            // Default retry after for rate limits
            Some(Duration::from_secs(30))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SageError;

    #[test]
    fn test_classify_http_errors() {
        assert_eq!(
            classify_error(&SageError::Http("connection timeout".into())),
            ErrorClass::Transient
        );
        assert_eq!(
            classify_error(&SageError::Http("429 rate limit".into())),
            ErrorClass::Transient
        );
        assert_eq!(
            classify_error(&SageError::Http("401 unauthorized".into())),
            ErrorClass::Permanent
        );
    }

    #[test]
    fn test_classify_llm_errors() {
        assert_eq!(
            classify_error(&SageError::Llm("rate limit exceeded".into())),
            ErrorClass::Transient
        );
        assert_eq!(
            classify_error(&SageError::Llm("context length exceeded".into())),
            ErrorClass::Permanent
        );
    }

    #[test]
    fn test_recoverable_error_creation() {
        let transient = RecoverableError::transient("network timeout");
        assert!(transient.is_retryable());

        let permanent = RecoverableError::permanent("invalid input");
        assert!(!permanent.is_retryable());
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new("LLMClient", "chat_stream")
            .with_metadata("model", "claude-3-opus")
            .with_metadata("attempt", "3");

        assert_eq!(ctx.component, Some("LLMClient".to_string()));
        assert_eq!(ctx.operation, Some("chat_stream".to_string()));
        assert_eq!(
            ctx.metadata.get("model"),
            Some(&"claude-3-opus".to_string())
        );
    }
}
