//! Error types for Sage Agent

use thiserror::Error;

/// Result type alias for Sage Agent operations
pub type SageResult<T> = Result<T, SageError>;

/// Main error type for Sage Agent
#[derive(Error, Debug, Clone)]
pub enum SageError {
    /// Configuration related errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// LLM client errors
    #[error("LLM error: {0}")]
    Llm(String),

    /// Tool execution errors
    #[error("Tool error: {tool_name}: {message}")]
    Tool { tool_name: String, message: String },

    /// Agent execution errors
    #[error("Agent error: {0}")]
    Agent(String),

    /// Cache errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(String),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(String),

    /// HTTP request errors
    #[error("HTTP error: {0}")]
    Http(String),

    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Task execution timeout
    #[error("Task execution timeout after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Task was cancelled
    #[error("Task was cancelled")]
    Cancelled,

    /// Generic error with context
    #[error("Error: {0}")]
    Other(String),
}

impl SageError {
    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a new LLM error
    pub fn llm(message: impl Into<String>) -> Self {
        Self::Llm(message.into())
    }

    /// Create a new tool error
    pub fn tool(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Tool {
            tool_name: tool_name.into(),
            message: message.into(),
        }
    }

    /// Create a new agent error
    pub fn agent(message: impl Into<String>) -> Self {
        Self::Agent(message.into())
    }

    /// Create a new cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache(message.into())
    }

    /// Create a new invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Create a new timeout error
    pub const fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }
}

impl From<anyhow::Error> for SageError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<std::io::Error> for SageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<serde_json::Error> for SageError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

impl From<reqwest::Error> for SageError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error.to_string())
    }
}

impl From<crate::mcp::McpError> for SageError {
    fn from(error: crate::mcp::McpError) -> Self {
        Self::Agent(format!("MCP error: {}", error))
    }
}

impl From<crate::agent::lifecycle::LifecycleError> for SageError {
    fn from(error: crate::agent::lifecycle::LifecycleError) -> Self {
        Self::Agent(format!("Lifecycle error: {}", error))
    }
}


