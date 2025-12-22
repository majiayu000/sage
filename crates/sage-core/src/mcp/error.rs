//! MCP error types

use thiserror::Error;
use crate::error::UnifiedError;

/// MCP-specific errors
///
/// Implements the `UnifiedError` trait for consistent error handling across crates.
#[derive(Debug, Error, Clone)]
pub enum McpError {
    /// Connection error
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        context: Option<String>,
    },

    /// Protocol error
    #[error("Protocol error: {message}")]
    Protocol {
        message: String,
        context: Option<String>,
    },

    /// Transport error
    #[error("Transport error: {message}")]
    Transport {
        message: String,
        context: Option<String>,
    },

    /// Server error with code
    #[error("Server error {code}: {message}")]
    Server {
        code: i32,
        message: String,
        context: Option<String>,
    },

    /// Tool not found
    #[error("Tool not found: {name}")]
    ToolNotFound {
        name: String,
        context: Option<String>,
    },

    /// Resource not found
    #[error("Resource not found: {resource}")]
    ResourceNotFound {
        resource: String,
        context: Option<String>,
    },

    /// Invalid request
    #[error("Invalid request: {message}")]
    InvalidRequest {
        message: String,
        context: Option<String>,
    },

    /// Timeout
    #[error("Request timeout after {seconds} seconds")]
    Timeout {
        seconds: u64,
        context: Option<String>,
    },

    /// Serialization error
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        context: Option<String>,
    },

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
    #[error("MCP error: {message}")]
    Other {
        message: String,
        context: Option<String>,
    },
}

impl McpError {
    /// Create a new Connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Protocol error
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Transport error
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Server error
    pub fn server(code: i32, message: impl Into<String>) -> Self {
        Self::Server {
            code,
            message: message.into(),
            context: None,
        }
    }

    /// Create a new ToolNotFound error
    pub fn tool_not_found(name: impl Into<String>) -> Self {
        Self::ToolNotFound {
            name: name.into(),
            context: None,
        }
    }

    /// Create a new ResourceNotFound error
    pub fn resource_not_found(resource: impl Into<String>) -> Self {
        Self::ResourceNotFound {
            resource: resource.into(),
            context: None,
        }
    }

    /// Create a new InvalidRequest error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout {
            seconds,
            context: None,
        }
    }

    /// Create a new Serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Other error
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            context: None,
        }
    }

    /// Add context to any MCP error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        let ctx = Some(context.into());
        match &mut self {
            Self::Connection { context: c, .. } => *c = ctx,
            Self::Protocol { context: c, .. } => *c = ctx,
            Self::Transport { context: c, .. } => *c = ctx,
            Self::Server { context: c, .. } => *c = ctx,
            Self::ToolNotFound { context: c, .. } => *c = ctx,
            Self::ResourceNotFound { context: c, .. } => *c = ctx,
            Self::InvalidRequest { context: c, .. } => *c = ctx,
            Self::Timeout { context: c, .. } => *c = ctx,
            Self::Serialization { context: c, .. } => *c = ctx,
            Self::Other { context: c, .. } => *c = ctx,
            Self::NotInitialized | Self::AlreadyInitialized | Self::Cancelled => {}
        }
        self
    }
}

impl UnifiedError for McpError {
    fn error_code(&self) -> &str {
        match self {
            Self::Connection { .. } => "MCP_CONNECTION",
            Self::Protocol { .. } => "MCP_PROTOCOL",
            Self::Transport { .. } => "MCP_TRANSPORT",
            Self::Server { .. } => "MCP_SERVER",
            Self::ToolNotFound { .. } => "MCP_TOOL_NOT_FOUND",
            Self::ResourceNotFound { .. } => "MCP_RESOURCE_NOT_FOUND",
            Self::InvalidRequest { .. } => "MCP_INVALID_REQUEST",
            Self::Timeout { .. } => "MCP_TIMEOUT",
            Self::Serialization { .. } => "MCP_SERIALIZATION",
            Self::NotInitialized => "MCP_NOT_INITIALIZED",
            Self::AlreadyInitialized => "MCP_ALREADY_INITIALIZED",
            Self::Cancelled => "MCP_CANCELLED",
            Self::Other { .. } => "MCP_OTHER",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Connection { message, .. } => message,
            Self::Protocol { message, .. } => message,
            Self::Transport { message, .. } => message,
            Self::Server { message, .. } => message,
            Self::ToolNotFound { name, .. } => name,
            Self::ResourceNotFound { resource, .. } => resource,
            Self::InvalidRequest { message, .. } => message,
            Self::Timeout { .. } => "Request timeout",
            Self::Serialization { message, .. } => message,
            Self::NotInitialized => "Client not initialized",
            Self::AlreadyInitialized => "Client already initialized",
            Self::Cancelled => "Operation cancelled",
            Self::Other { message, .. } => message,
        }
    }

    fn context(&self) -> Option<&str> {
        match self {
            Self::Connection { context, .. } => context.as_deref(),
            Self::Protocol { context, .. } => context.as_deref(),
            Self::Transport { context, .. } => context.as_deref(),
            Self::Server { context, .. } => context.as_deref(),
            Self::ToolNotFound { context, .. } => context.as_deref(),
            Self::ResourceNotFound { context, .. } => context.as_deref(),
            Self::InvalidRequest { context, .. } => context.as_deref(),
            Self::Timeout { context, .. } => context.as_deref(),
            Self::Serialization { context, .. } => context.as_deref(),
            Self::Other { context, .. } => context.as_deref(),
            Self::NotInitialized | Self::AlreadyInitialized | Self::Cancelled => None,
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Connection { .. } | Self::Transport { .. } | Self::Timeout { .. } | Self::Server { .. }
        )
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization(err.to_string())
    }
}

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        Self::transport(err.to_string())
    }
}

impl From<crate::error::SageError> for McpError {
    fn from(err: crate::error::SageError) -> Self {
        Self::other(err.to_string())
    }
}
