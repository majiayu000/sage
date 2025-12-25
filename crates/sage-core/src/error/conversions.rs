//! From trait implementations for SageError conversions

use super::types::SageError;

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
        use crate::error::UnifiedError;
        Self::agent_with_context(
            format!("MCP error: {}", error),
            format!("MCP error code: {}", error.error_code()),
        )
    }
}

impl From<crate::agent::lifecycle::LifecycleError> for SageError {
    fn from(error: crate::agent::lifecycle::LifecycleError) -> Self {
        use crate::error::UnifiedError;
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
            crate::workspace::WorkspaceError::DirectoryNotFound(path) => Self::not_found_resource(
                format!("Directory not found: {}", path.display()),
                "directory",
            ),
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
            SandboxError::Internal(msg) => Self::agent(format!("Sandbox internal error: {}", msg)),
        }
    }
}
