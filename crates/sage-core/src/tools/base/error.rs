//! Error types for tool operations

use crate::error::SageError;

/// Error type for tool operations
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Invalid arguments provided to the tool
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Tool execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Tool not found
    #[error("Tool not found: {0}")]
    NotFound(String),

    /// Tool timeout
    #[error("Tool execution timeout")]
    Timeout,

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Cancelled
    #[error("Tool execution cancelled")]
    Cancelled,

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

impl crate::error::UnifiedError for ToolError {
    fn error_code(&self) -> &str {
        match self {
            ToolError::InvalidArguments(_) => "TOOL_INVALID_ARGS",
            ToolError::ExecutionFailed(_) => "TOOL_EXEC_FAILED",
            ToolError::NotFound(_) => "TOOL_NOT_FOUND",
            ToolError::Timeout => "TOOL_TIMEOUT",
            ToolError::PermissionDenied(_) => "TOOL_PERMISSION_DENIED",
            ToolError::ValidationFailed(_) => "TOOL_VALIDATION_FAILED",
            ToolError::Io(_) => "TOOL_IO_ERROR",
            ToolError::Json(_) => "TOOL_JSON_ERROR",
            ToolError::Cancelled => "TOOL_CANCELLED",
            ToolError::Other(_) => "TOOL_OTHER",
        }
    }

    fn message(&self) -> &str {
        match self {
            ToolError::InvalidArguments(msg) => msg,
            ToolError::ExecutionFailed(msg) => msg,
            ToolError::NotFound(name) => name,
            ToolError::Timeout => "Tool execution timeout",
            ToolError::PermissionDenied(msg) => msg,
            ToolError::ValidationFailed(msg) => msg,
            ToolError::Io(_) => "IO error occurred",
            ToolError::Json(_) => "JSON error occurred",
            ToolError::Cancelled => "Tool execution cancelled",
            ToolError::Other(msg) => msg,
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(self, ToolError::Timeout | ToolError::Io(_))
    }
}

impl From<ToolError> for SageError {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::NotFound(name) => SageError::tool(name, "Tool not found"),
            ToolError::InvalidArguments(msg) => SageError::tool("unknown", msg),
            ToolError::ExecutionFailed(msg) => SageError::tool("unknown", msg),
            ToolError::Timeout => SageError::tool("unknown", "Tool execution timeout"),
            ToolError::PermissionDenied(msg) => SageError::tool("unknown", msg),
            ToolError::ValidationFailed(msg) => SageError::tool("unknown", msg),
            ToolError::Io(err) => SageError::tool("unknown", err.to_string()),
            ToolError::Json(err) => SageError::tool("unknown", err.to_string()),
            ToolError::Cancelled => SageError::tool("unknown", "Cancelled"),
            ToolError::Other(msg) => SageError::tool("unknown", msg),
        }
    }
}
