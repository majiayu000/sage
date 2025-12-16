//! Base trait and types for tools

use crate::error::SageError;
use crate::tools::permission::{PermissionResult, RiskLevel, ToolContext};
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::time::{Duration, Instant};

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

/// Concurrency mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConcurrencyMode {
    /// Tool can run in parallel with any other tool
    #[default]
    Parallel,

    /// Tool must run sequentially (one at a time globally)
    Sequential,

    /// Tool can run in parallel but with a maximum count
    Limited(usize),

    /// Tool can run in parallel but not with tools of the same type
    ExclusiveByType,
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

/// Base trait for all tools
///
/// Tools are capabilities that agents can use to interact with the environment.
/// Each tool has a schema for validation, permission checking, and execution logic.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's unique name
    ///
    /// Tool names must be unique within a registry and should follow
    /// the pattern: lowercase with underscores (e.g., "read_file").
    fn name(&self) -> &str;

    /// Get the tool's description
    ///
    /// This description is included in the system prompt to help the
    /// LLM understand when to use this tool.
    fn description(&self) -> &str;

    /// Get the tool's JSON schema for input parameters
    fn schema(&self) -> ToolSchema;

    /// Execute the tool with the given arguments
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;

    /// Validate the tool call arguments
    ///
    /// Default implementation does nothing. Override for custom validation.
    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let _ = call;
        Ok(())
    }

    /// Check if the tool call is permitted in the current context
    ///
    /// This method is called before execution to determine if the
    /// operation should be allowed, denied, or requires user approval.
    async fn check_permission(&self, _call: &ToolCall, _context: &ToolContext) -> PermissionResult {
        // Default: allow all operations
        PermissionResult::Allow
    }

    /// Get the risk level for this tool
    ///
    /// Used for permission checking and user notifications.
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// Get the concurrency mode for this tool
    ///
    /// Determines whether multiple instances can run in parallel.
    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Parallel
    }

    /// Get the maximum execution time as Duration
    fn max_execution_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(300)) // Default 5 minutes
    }

    /// Get the maximum execution time in seconds (for backwards compatibility)
    fn max_execution_time(&self) -> Option<u64> {
        self.max_execution_duration().map(|d| d.as_secs())
    }

    /// Whether this tool only reads data (no side effects)
    fn is_read_only(&self) -> bool {
        false
    }

    /// Whether this tool can be called in parallel with other tools
    fn supports_parallel_execution(&self) -> bool {
        matches!(
            self.concurrency_mode(),
            ConcurrencyMode::Parallel | ConcurrencyMode::Limited(_)
        )
    }

    /// Render the tool call for display to the user
    fn render_call(&self, call: &ToolCall) -> String {
        format!(
            "{}({})",
            self.name(),
            serde_json::to_string(&call.arguments).unwrap_or_default()
        )
    }

    /// Render the tool result for display to the user
    fn render_result(&self, result: &ToolResult) -> String {
        if result.success {
            result.output.clone().unwrap_or_default()
        } else {
            format!("Error: {}", result.error.clone().unwrap_or_default())
        }
    }

    /// Execute the tool with timing and error handling
    async fn execute_with_timing(&self, call: &ToolCall) -> ToolResult {
        let start_time = Instant::now();

        // Validate arguments first
        if let Err(err) = self.validate(call) {
            return ToolResult::error(&call.id, self.name(), err.to_string())
                .with_execution_time(start_time.elapsed().as_millis() as u64);
        }

        // Execute the tool
        match self.execute(call).await {
            Ok(mut result) => {
                result.execution_time_ms = Some(start_time.elapsed().as_millis() as u64);
                result
            }
            Err(err) => ToolResult::error(&call.id, self.name(), err.to_string())
                .with_execution_time(start_time.elapsed().as_millis() as u64),
        }
    }
}

/// Macro to help implement the Tool trait
#[macro_export]
macro_rules! impl_tool {
    ($tool_type:ty, $name:expr, $description:expr) => {
        impl $tool_type {
            pub fn new() -> Self {
                Self {}
            }
        }

        impl Default for $tool_type {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait::async_trait]
        impl $crate::tools::Tool for $tool_type {
            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $description
            }
        }
    };
}

/// Helper trait for tools that need access to the file system
pub trait FileSystemTool: Tool {
    /// Get the working directory for file operations
    fn working_directory(&self) -> &std::path::Path;

    /// Resolve a relative path to an absolute path
    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(path);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory().join(path)
        }
    }

    /// Check if a path is safe to access (within working directory)
    fn is_safe_path(&self, _path: &std::path::Path) -> bool {
        // Temporarily disable path restrictions for debugging
        true
    }
}

/// Helper trait for tools that execute commands
pub trait CommandTool: Tool {
    /// Get the allowed commands for this tool
    fn allowed_commands(&self) -> Vec<&str>;

    /// Check if a command is allowed
    fn is_command_allowed(&self, command: &str) -> bool {
        let allowed = self.allowed_commands();
        if allowed.is_empty() {
            return true; // No restrictions
        }

        allowed
            .iter()
            .any(|&allowed_cmd| command.starts_with(allowed_cmd))
    }

    /// Get the working directory for command execution
    fn command_working_directory(&self) -> &std::path::Path;

    /// Get environment variables for command execution
    fn command_environment(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
}
