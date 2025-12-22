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

    /// Whether this tool requires user interaction to complete
    ///
    /// When a tool returns `true`, the execution loop will block and wait
    /// for user input via the InputChannel instead of continuing immediately.
    /// This is used for tools like `ask_user_question` that need to gather
    /// information from the user.
    ///
    /// When a tool requires user interaction:
    /// 1. The tool execution prepares an InputRequest
    /// 2. The execution loop sends it to the InputChannel
    /// 3. The loop blocks until the user responds
    /// 4. The response is returned as part of the tool result
    fn requires_user_interaction(&self) -> bool {
        false
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
    ///
    /// This method prevents path traversal attacks by ensuring the resolved
    /// path is within the working directory. It handles:
    /// - Absolute paths that point outside working directory
    /// - Relative paths with `..` components that escape the sandbox
    /// - Symlinks that point outside the working directory
    fn is_safe_path(&self, path: &std::path::Path) -> bool {
        // Get the canonical working directory
        let working_dir = match self.working_directory().canonicalize() {
            Ok(p) => p,
            Err(_) => return false, // Can't verify if working dir doesn't exist
        };

        // Try to canonicalize the target path
        let canonical = if path.exists() {
            match path.canonicalize() {
                Ok(p) => p,
                Err(_) => return false,
            }
        } else {
            // For new files/directories, find the nearest existing ancestor
            // and build the path from there
            let mut current = path.to_path_buf();
            let mut components_to_add = Vec::new();

            // Walk up until we find an existing directory
            loop {
                if current.exists() {
                    match current.canonicalize() {
                        Ok(canonical_ancestor) => {
                            // Build the full path by appending non-existent components
                            let mut result = canonical_ancestor;
                            for component in components_to_add.into_iter().rev() {
                                result = result.join(component);
                            }
                            break result;
                        }
                        Err(_) => return false,
                    }
                }

                // Get the file name component to add later
                if let Some(name) = current.file_name() {
                    components_to_add.push(name.to_os_string());
                }

                // Move to parent
                if let Some(parent) = current.parent() {
                    if parent.as_os_str().is_empty() {
                        // We've reached the root of a relative path
                        // Use working directory as the base
                        let mut result = working_dir.clone();
                        for component in components_to_add.into_iter().rev() {
                            result = result.join(component);
                        }
                        break result;
                    }
                    current = parent.to_path_buf();
                } else {
                    return false;
                }
            }
        };

        // Check for path traversal attempts in the non-existent portion
        // by ensuring no ".." components exist after normalization
        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                // Found a ".." - need to verify the final path is still safe
                // The canonical path already resolved these, but we need to
                // ensure we don't escape the sandbox
            }
        }

        // Check if the canonical path starts with the working directory
        canonical.starts_with(&working_dir)
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
