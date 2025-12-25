//! Core error types and traits for Sage Agent

use thiserror::Error;

/// Result type alias for Sage Agent operations
pub type SageResult<T> = Result<T, SageError>;

/// Unified error trait that all Sage errors implement.
///
/// This trait ensures consistent error handling across all crates by providing:
/// - error_code(): Unique code for programmatic error identification
/// - message(): Human-readable error message
/// - context(): Optional additional context
/// - source_error(): Optional underlying error
pub trait UnifiedError: std::error::Error + Send + Sync {
    /// Get the error code for programmatic handling
    fn error_code(&self) -> &str;

    /// Get the human-readable error message
    fn message(&self) -> &str;

    /// Get optional context about the error
    fn context(&self) -> Option<&str> {
        None
    }

    /// Get the underlying source error if any
    fn source_error(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source()
    }

    /// Check if this error is retryable
    fn is_retryable(&self) -> bool {
        false
    }
}

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to an error
    fn context<C: std::fmt::Display>(self, context: C) -> SageResult<T>;

    /// Add context lazily (only evaluated on error)
    fn with_context<C: std::fmt::Display, F: FnOnce() -> C>(self, f: F) -> SageResult<T>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn context<C: std::fmt::Display>(self, context: C) -> SageResult<T> {
        self.map_err(|e| SageError::other(format!("{}: {}", context, e)))
    }

    fn with_context<C: std::fmt::Display, F: FnOnce() -> C>(self, f: F) -> SageResult<T> {
        self.map_err(|e| SageError::other(format!("{}: {}", f(), e)))
    }
}

/// Extension trait for adding context to Option types
pub trait OptionExt<T> {
    /// Convert Option to Result with context message
    fn context<C: std::fmt::Display>(self, context: C) -> SageResult<T>;

    /// Convert Option to Result with lazy context message
    fn with_context<C: std::fmt::Display, F: FnOnce() -> C>(self, f: F) -> SageResult<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn context<C: std::fmt::Display>(self, context: C) -> SageResult<T> {
        self.ok_or_else(|| SageError::other(context.to_string()))
    }

    fn with_context<C: std::fmt::Display, F: FnOnce() -> C>(self, f: F) -> SageResult<T> {
        self.ok_or_else(|| SageError::other(f().to_string()))
    }
}

/// Main error type for Sage Agent
///
/// This enum implements the `UnifiedError` trait to provide consistent error handling
/// across all crates. Each variant includes contextual information where relevant.
#[derive(Error, Debug, Clone)]
pub enum SageError {
    /// Configuration related errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<SageError>>,
        context: Option<String>,
    },

    /// LLM client errors
    #[error("LLM error: {message}")]
    Llm {
        message: String,
        provider: Option<String>,
        context: Option<String>,
    },

    /// Tool execution errors
    #[error("Tool error: {tool_name}: {message}")]
    Tool {
        tool_name: String,
        message: String,
        context: Option<String>,
    },

    /// Agent execution errors
    #[error("Agent error: {message}")]
    Agent {
        message: String,
        context: Option<String>,
    },

    /// Cache errors
    #[error("Cache error: {message}")]
    Cache {
        message: String,
        context: Option<String>,
    },

    /// IO errors
    #[error("IO error: {message}")]
    Io {
        message: String,
        path: Option<String>,
        context: Option<String>,
    },

    /// JSON serialization/deserialization errors
    #[error("JSON error: {message}")]
    Json {
        message: String,
        context: Option<String>,
    },

    /// HTTP request errors
    #[error("HTTP error: {message}")]
    Http {
        message: String,
        url: Option<String>,
        status_code: Option<u16>,
        context: Option<String>,
    },

    /// Invalid input errors
    #[error("Invalid input: {message}")]
    InvalidInput {
        message: String,
        field: Option<String>,
        context: Option<String>,
    },

    /// Task execution timeout
    #[error("Task execution timeout after {seconds} seconds")]
    Timeout {
        seconds: u64,
        context: Option<String>,
    },

    /// Task was cancelled
    #[error("Task was cancelled")]
    Cancelled,

    /// Storage/persistence errors
    #[error("Storage error: {message}")]
    Storage {
        message: String,
        context: Option<String>,
    },

    /// Resource not found
    #[error("Not found: {message}")]
    NotFound {
        message: String,
        resource_type: Option<String>,
        context: Option<String>,
    },

    /// Generic error with context
    #[error("Error: {message}")]
    Other {
        message: String,
        context: Option<String>,
    },
}
