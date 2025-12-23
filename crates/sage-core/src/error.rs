//! Error types for Sage Agent
//!
//! This module provides a unified error handling system across all Sage Agent crates.
//! All errors implement the `UnifiedError` trait which provides consistent fields:
//! - error_code: A unique identifier for programmatic error handling
//! - message: Human-readable error message
//! - context: Optional additional context about where/why the error occurred
//! - source: Optional underlying error that caused this error

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

impl SageError {
    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
            context: None,
        }
    }

    /// Create a configuration error with context
    pub fn config_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
            context: Some(context.into()),
        }
    }

    /// Create a new LLM error
    pub fn llm(message: impl Into<String>) -> Self {
        Self::Llm {
            message: message.into(),
            provider: None,
            context: None,
        }
    }

    /// Create an LLM error with provider
    pub fn llm_with_provider(message: impl Into<String>, provider: impl Into<String>) -> Self {
        Self::Llm {
            message: message.into(),
            provider: Some(provider.into()),
            context: None,
        }
    }

    /// Create a new tool error
    pub fn tool(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Tool {
            tool_name: tool_name.into(),
            message: message.into(),
            context: None,
        }
    }

    /// Create a tool error with context
    pub fn tool_with_context(
        tool_name: impl Into<String>,
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::Tool {
            tool_name: tool_name.into(),
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create a new agent error
    pub fn agent(message: impl Into<String>) -> Self {
        Self::Agent {
            message: message.into(),
            context: None,
        }
    }

    /// Create an agent error with context
    pub fn agent_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::Agent {
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create a new cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field: None,
            context: None,
        }
    }

    /// Create an invalid input error with field
    pub fn invalid_input_field(
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field: Some(field.into()),
            context: None,
        }
    }

    /// Create a new timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout {
            seconds,
            context: None,
        }
    }

    /// Create a new storage error
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            resource_type: None,
            context: None,
        }
    }

    /// Create a not found error with resource type
    pub fn not_found_resource(
        message: impl Into<String>,
        resource_type: impl Into<String>,
    ) -> Self {
        Self::NotFound {
            message: message.into(),
            resource_type: Some(resource_type.into()),
            context: None,
        }
    }

    /// Create an execution error (alias for agent error)
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Agent {
            message: message.into(),
            context: None,
        }
    }

    /// Create an IO error with message
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            path: None,
            context: None,
        }
    }

    /// Create an IO error with path
    pub fn io_with_path(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            path: Some(path.into()),
            context: None,
        }
    }

    /// Create a JSON error with message
    pub fn json(message: impl Into<String>) -> Self {
        Self::Json {
            message: message.into(),
            context: None,
        }
    }

    /// Create an HTTP error with message
    pub fn http(message: impl Into<String>) -> Self {
        Self::Http {
            message: message.into(),
            url: None,
            status_code: None,
            context: None,
        }
    }

    /// Create an HTTP error with status code
    pub fn http_with_status(message: impl Into<String>, status_code: u16) -> Self {
        Self::Http {
            message: message.into(),
            url: None,
            status_code: Some(status_code),
            context: None,
        }
    }

    /// Create a generic error with context
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            context: None,
        }
    }

    /// Add context to any error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        let ctx = Some(context.into());
        match &mut self {
            Self::Config { context: c, .. } => *c = ctx,
            Self::Llm { context: c, .. } => *c = ctx,
            Self::Tool { context: c, .. } => *c = ctx,
            Self::Agent { context: c, .. } => *c = ctx,
            Self::Cache { context: c, .. } => *c = ctx,
            Self::Io { context: c, .. } => *c = ctx,
            Self::Json { context: c, .. } => *c = ctx,
            Self::Http { context: c, .. } => *c = ctx,
            Self::InvalidInput { context: c, .. } => *c = ctx,
            Self::Timeout { context: c, .. } => *c = ctx,
            Self::Storage { context: c, .. } => *c = ctx,
            Self::NotFound { context: c, .. } => *c = ctx,
            Self::Other { context: c, .. } => *c = ctx,
            Self::Cancelled => {}
        }
        self
    }
}

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

impl From<anyhow::Error> for SageError {
    fn from(error: anyhow::Error) -> Self {
        Self::other(error.to_string())
    }
}

impl From<std::io::Error> for SageError {
    fn from(error: std::io::Error) -> Self {
        Self::io(error.to_string())
    }
}

impl From<serde_json::Error> for SageError {
    fn from(error: serde_json::Error) -> Self {
        Self::json(error.to_string())
    }
}

impl From<reqwest::Error> for SageError {
    fn from(error: reqwest::Error) -> Self {
        let status_code = error.status().map(|s| s.as_u16());
        let url = error.url().map(|u| u.to_string());
        Self::Http {
            message: error.to_string(),
            url,
            status_code,
            context: None,
        }
    }
}

impl From<crate::mcp::McpError> for SageError {
    fn from(error: crate::mcp::McpError) -> Self {
        Self::agent_with_context(
            format!("MCP error: {}", error),
            format!("MCP error code: {}", error.error_code()),
        )
    }
}

impl From<crate::agent::lifecycle::LifecycleError> for SageError {
    fn from(error: crate::agent::lifecycle::LifecycleError) -> Self {
        Self::agent_with_context(
            format!("Lifecycle error: {}", error),
            format!("Lifecycle error code: {}", error.error_code()),
        )
    }
}

impl From<crate::validation::ValidationError> for SageError {
    fn from(error: crate::validation::ValidationError) -> Self {
        let message = error.all_errors().join("; ");
        Self::InvalidInput {
            message,
            field: None,
            context: Some(format!("{} validation error(s)", error.error_count())),
        }
    }
}

impl From<crate::workspace::WorkspaceError> for SageError {
    fn from(error: crate::workspace::WorkspaceError) -> Self {
        match error {
            crate::workspace::WorkspaceError::DirectoryNotFound(path) => {
                Self::not_found_resource(
                    format!("Directory not found: {}", path.display()),
                    "directory",
                )
            }
            crate::workspace::WorkspaceError::NotADirectory(path) => {
                Self::invalid_input(format!("Not a directory: {}", path.display()))
            }
            crate::workspace::WorkspaceError::Io(err) => Self::io(err.to_string()),
            crate::workspace::WorkspaceError::AnalysisFailed(msg) => {
                Self::agent_with_context("Workspace analysis failed", msg)
            }
        }
    }
}

impl From<crate::storage::DatabaseError> for SageError {
    fn from(error: crate::storage::DatabaseError) -> Self {
        use crate::storage::DatabaseError as DbErr;
        match error {
            DbErr::Connection(msg) | DbErr::Query(msg) | DbErr::Transaction(msg) => {
                Self::storage(msg)
            }
            DbErr::Serialization(msg) => Self::json(msg),
            DbErr::NotFound(msg) => Self::not_found_resource(msg, "database record"),
            DbErr::Constraint(msg) => {
                Self::invalid_input_field("Database constraint violation", msg)
            }
            DbErr::Migration(msg) => Self::storage(format!("Database migration failed: {}", msg)),
            DbErr::NotAvailable(msg) => Self::storage(format!("Database not available: {}", msg)),
            DbErr::Io(err) => Self::io(err.to_string()),
            DbErr::Internal(msg) => Self::storage(format!("Internal database error: {}", msg)),
        }
    }
}

impl From<crate::prompts::RenderError> for SageError {
    fn from(error: crate::prompts::RenderError) -> Self {
        use crate::prompts::RenderError;
        match error {
            RenderError::MissingRequired(var) => {
                Self::invalid_input_field(format!("Missing required variable: {}", var), var)
            }
            RenderError::InvalidVariable(var) => {
                Self::invalid_input_field(format!("Invalid variable: {}", var), var)
            }
            RenderError::ParseError(msg) => Self::other(format!("Template parse error: {}", msg)),
        }
    }
}

impl From<crate::sandbox::SandboxError> for SageError {
    fn from(error: crate::sandbox::SandboxError) -> Self {
        use crate::sandbox::SandboxError;
        match error {
            SandboxError::ResourceLimitExceeded {
                resource,
                current,
                limit,
            } => Self::agent_with_context(
                format!("Resource limit exceeded: {}", resource),
                format!("current: {}, limit: {}", current, limit),
            ),
            SandboxError::PathAccessDenied { path } => {
                Self::tool("sandbox", format!("Path access denied: {}", path))
            }
            SandboxError::CommandNotAllowed { command } => {
                Self::tool("sandbox", format!("Command not allowed: {}", command))
            }
            SandboxError::NetworkAccessDenied { host } => {
                Self::tool("sandbox", format!("Network access denied: {}", host))
            }
            SandboxError::Timeout(duration) => Self::timeout(duration.as_secs()),
            SandboxError::InitializationFailed(msg) => {
                Self::agent(format!("Sandbox initialization failed: {}", msg))
            }
            SandboxError::SpawnFailed(msg) => {
                Self::agent(format!("Failed to spawn sandboxed process: {}", msg))
            }
            SandboxError::InvalidConfig(msg) => {
                Self::config(format!("Invalid sandbox configuration: {}", msg))
            }
            SandboxError::PermissionDenied(msg) => {
                Self::tool("sandbox", format!("Permission denied: {}", msg))
            }
            SandboxError::Internal(msg) => {
                Self::agent(format!("Sandbox internal error: {}", msg))
            }
        }
    }
}
