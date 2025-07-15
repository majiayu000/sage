//! Base trait and types for tools

use crate::error::SageError;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::time::Instant;

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
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

impl From<ToolError> for SageError {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::NotFound(name) => SageError::tool(name, "Tool not found"),
            ToolError::InvalidArguments(msg) => SageError::tool("unknown", msg),
            ToolError::ExecutionFailed(msg) => SageError::tool("unknown", msg),
            ToolError::Timeout => SageError::tool("unknown", "Tool execution timeout"),
            ToolError::PermissionDenied(msg) => SageError::tool("unknown", msg),
            ToolError::Io(err) => SageError::tool("unknown", err.to_string()),
            ToolError::Json(err) => SageError::tool("unknown", err.to_string()),
            ToolError::Other(msg) => SageError::tool("unknown", msg),
        }
    }
}

/// Base trait for all tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;

    /// Get the tool's description
    fn description(&self) -> &str;

    /// Get the tool's JSON schema
    fn schema(&self) -> ToolSchema;

    /// Execute the tool with the given arguments
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;

    /// Validate the tool call arguments (optional override)
    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Default implementation - tools can override for custom validation
        let _ = call; // Suppress unused parameter warning
        Ok(())
    }

    /// Get the maximum execution time for this tool in seconds (optional)
    fn max_execution_time(&self) -> Option<u64> {
        Some(300) // Default 5 minutes
    }

    /// Whether this tool can be called in parallel with other tools
    fn supports_parallel_execution(&self) -> bool {
        true
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
            Err(err) => {
                ToolResult::error(&call.id, self.name(), err.to_string())
                    .with_execution_time(start_time.elapsed().as_millis() as u64)
            }
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
        
        allowed.iter().any(|&allowed_cmd| {
            command.starts_with(allowed_cmd)
        })
    }
    
    /// Get the working directory for command execution
    fn command_working_directory(&self) -> &std::path::Path;
    
    /// Get environment variables for command execution
    fn command_environment(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
}
