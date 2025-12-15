//! MCP error types

use thiserror::Error;

/// MCP-specific errors
#[derive(Debug, Error, Clone)]
pub enum McpError {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Server error with code
    #[error("Server error {code}: {message}")]
    Server { code: i32, message: String },

    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Timeout
    #[error("Request timeout after {0} seconds")]
    Timeout(u64),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Not initialized
    #[error("Client not initialized")]
    NotInitialized,

    /// Already initialized
    #[error("Client already initialized")]
    AlreadyInitialized,

    /// Cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Other error
    #[error("MCP error: {0}")]
    Other(String),
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        Self::Transport(err.to_string())
    }
}

impl From<crate::error::SageError> for McpError {
    fn from(err: crate::error::SageError) -> Self {
        Self::Other(err.to_string())
    }
}
