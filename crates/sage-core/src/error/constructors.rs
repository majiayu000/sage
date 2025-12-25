//! Constructor methods for SageError

use super::types::SageError;

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
    pub fn invalid_input_field(message: impl Into<String>, field: impl Into<String>) -> Self {
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
