//! Error recovery system for Sage Agent
//!
//! This module provides error recovery capabilities:
//! - Error classification (transient vs permanent)
//! - Retry configuration
//! - Circuit breaker pattern for failing dependencies
//! - Supervision and error isolation

pub mod circuit_breaker;
pub mod rate_limiter;
pub mod retry;
pub mod supervisor;

pub use retry::RetryConfig;

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
}

impl RecoverableError {
    /// Create a permanent error
    pub fn permanent(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: ErrorClass::Permanent,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self.class, ErrorClass::Transient | ErrorClass::Unknown)
    }
}

/// Classify common errors into error classes
pub fn classify_error(error: &crate::error::SageError) -> ErrorClass {
    use crate::error::SageError;

    match error {
        SageError::Http { message: msg, .. } => {
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
                ErrorClass::Transient
            } else {
                ErrorClass::Unknown
            }
        }
        SageError::Io { message: msg, .. } => {
            if msg.contains("permission denied") || msg.contains("not found") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Transient
            }
        }
        SageError::Llm { message: msg, .. } => {
            if msg.contains("rate limit") || msg.contains("overloaded") {
                ErrorClass::Transient
            } else if msg.contains("invalid") || msg.contains("context length") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Unknown
            }
        }
        SageError::Timeout { .. } => ErrorClass::Transient,
        SageError::Cancelled => ErrorClass::Permanent,
        SageError::Config { .. } | SageError::InvalidInput { .. } => ErrorClass::Permanent,
        SageError::Json { .. } => ErrorClass::Permanent,
        SageError::Tool { message, .. } => {
            if message.contains("timeout") || message.contains("temporarily") {
                ErrorClass::Transient
            } else {
                ErrorClass::Unknown
            }
        }
        SageError::Agent { .. } | SageError::Cache { .. } => ErrorClass::Unknown,
        SageError::Storage { message: msg, .. } => {
            if msg.contains("permission denied") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Transient
            }
        }
        SageError::NotFound { .. } => ErrorClass::Permanent,
        SageError::Other { .. } => ErrorClass::Unknown,
    }
}

/// Convert a SageError to a RecoverableError
pub fn to_recoverable(error: &crate::error::SageError) -> RecoverableError {
    let class = classify_error(error);
    RecoverableError {
        message: error.to_string(),
        class,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SageError;

    #[test]
    fn test_classify_http_errors() {
        assert_eq!(
            classify_error(&SageError::http("connection timeout")),
            ErrorClass::Transient
        );
        assert_eq!(
            classify_error(&SageError::http("429 rate limit")),
            ErrorClass::Transient
        );
        assert_eq!(
            classify_error(&SageError::http("401 unauthorized")),
            ErrorClass::Permanent
        );
    }

    #[test]
    fn test_classify_llm_errors() {
        assert_eq!(
            classify_error(&SageError::llm("rate limit exceeded")),
            ErrorClass::Transient
        );
        assert_eq!(
            classify_error(&SageError::llm("context length exceeded")),
            ErrorClass::Permanent
        );
    }

    #[test]
    fn test_recoverable_error_creation() {
        let permanent = RecoverableError::permanent("invalid input");
        assert!(!permanent.is_retryable());
    }
}
