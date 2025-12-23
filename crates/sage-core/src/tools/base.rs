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

    /// Get the tool's JSON schema for input parameters.
    ///
    /// The schema defines the structure and types of arguments this tool accepts.
    /// It is used for validation and to generate prompts for LLMs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn description(&self) -> &str { "A custom tool" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    /// ```
    fn schema(&self) -> ToolSchema;

    /// Execute the tool with the given arguments.
    ///
    /// This is the core method that implements the tool's functionality.
    /// It receives a `ToolCall` containing validated arguments and should
    /// return a `ToolResult` with the execution outcome.
    ///
    /// # Arguments
    ///
    /// * `call` - The tool call containing arguments and metadata
    ///
    /// # Errors
    ///
    /// Returns `ToolError` if:
    /// - Arguments are invalid or missing
    /// - Execution fails due to system errors
    /// - Required resources are unavailable
    /// - Permissions are denied
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// use sage_core::tools::base::ToolError;
    /// use sage_core::tools::types::{ToolCall, ToolResult};
    /// use async_trait::async_trait;
    ///
    /// struct EchoTool;
    ///
    /// #[async_trait]
    /// impl Tool for EchoTool {
    ///     fn name(&self) -> &str { "echo" }
    ///     fn description(&self) -> &str { "Echoes input" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         let message = call.arguments.get("message")
    ///             .and_then(|v| v.as_str())
    ///             .ok_or_else(|| ToolError::InvalidArguments("message required".into()))?;
    ///
    ///         Ok(ToolResult::success(&call.id, self.name(), message))
    ///     }
    /// }
    /// ```
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;

    /// Validate the tool call arguments.
    ///
    /// This method is called before `execute()` to verify that arguments
    /// are valid. The default implementation accepts all arguments.
    ///
    /// # Arguments
    ///
    /// * `call` - The tool call to validate
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidArguments` if validation fails.
    /// Returns `ToolError::ValidationFailed` for complex validation errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// use sage_core::tools::base::ToolError;
    /// use sage_core::tools::types::{ToolCall, ToolResult};
    /// use async_trait::async_trait;
    ///
    /// struct PositiveNumberTool;
    ///
    /// #[async_trait]
    /// impl Tool for PositiveNumberTool {
    ///     fn name(&self) -> &str { "positive" }
    ///     fn description(&self) -> &str { "Requires positive number" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
    ///         let num = call.arguments.get("number")
    ///             .and_then(|v| v.as_i64())
    ///             .ok_or_else(|| ToolError::InvalidArguments("number required".into()))?;
    ///
    ///         if num <= 0 {
    ///             return Err(ToolError::ValidationFailed("number must be positive".into()));
    ///         }
    ///         Ok(())
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "valid"))
    ///     }
    /// }
    /// ```
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

    /// Get the maximum execution time as Duration.
    ///
    /// Tools that exceed this duration will be terminated. The default
    /// is 5 minutes (300 seconds). Return `None` for no timeout.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::Tool;
    /// use std::time::Duration;
    /// # use sage_core::tools::{ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    ///
    /// struct FastTool;
    ///
    /// #[async_trait]
    /// impl Tool for FastTool {
    ///     fn name(&self) -> &str { "fast" }
    ///     fn description(&self) -> &str { "Fast operation" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     fn max_execution_duration(&self) -> Option<Duration> {
    ///         Some(Duration::from_secs(10))  // 10 second timeout
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    /// ```
    fn max_execution_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(300)) // Default 5 minutes
    }

    /// Get the maximum execution time in seconds.
    ///
    /// This is a convenience method that converts `max_execution_duration()`
    /// to seconds. Maintained for backwards compatibility.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::Tool;
    /// # use sage_core::tools::{ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn description(&self) -> &str { "A tool" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    ///
    /// # fn example() {
    /// let tool = MyTool;
    /// assert_eq!(tool.max_execution_time(), Some(300));  // Default 5 minutes
    /// # }
    /// ```
    fn max_execution_time(&self) -> Option<u64> {
        self.max_execution_duration().map(|d| d.as_secs())
    }

    /// Whether this tool only reads data without side effects.
    ///
    /// Read-only tools are safer to execute and may be given different
    /// permission policies. Examples include file reading, listing directories,
    /// or querying databases without modifications.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::Tool;
    /// # use sage_core::tools::{ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    ///
    /// struct ReadFileTool;
    ///
    /// #[async_trait]
    /// impl Tool for ReadFileTool {
    ///     fn name(&self) -> &str { "read_file" }
    ///     fn description(&self) -> &str { "Read file contents" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     fn is_read_only(&self) -> bool {
    ///         true  // Reading doesn't modify anything
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "file contents"))
    ///     }
    /// }
    /// ```
    fn is_read_only(&self) -> bool {
        false
    }

    /// Whether this tool can be called in parallel with other tools.
    ///
    /// This is determined by the tool's `concurrency_mode()`. Tools with
    /// `Parallel` or `Limited` modes support parallel execution, while
    /// `Sequential` and `ExclusiveByType` modes do not.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// use sage_core::tools::base::{ToolError, ConcurrencyMode};
    /// use sage_core::tools::types::{ToolCall, ToolResult};
    /// use async_trait::async_trait;
    ///
    /// struct ParallelTool;
    ///
    /// #[async_trait]
    /// impl Tool for ParallelTool {
    ///     fn name(&self) -> &str { "parallel" }
    ///     fn description(&self) -> &str { "Can run in parallel" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     fn concurrency_mode(&self) -> ConcurrencyMode {
    ///         ConcurrencyMode::Parallel
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    ///
    /// # fn example() {
    /// let tool = ParallelTool;
    /// assert!(tool.supports_parallel_execution());
    /// # }
    /// ```
    fn supports_parallel_execution(&self) -> bool {
        matches!(
            self.concurrency_mode(),
            ConcurrencyMode::Parallel | ConcurrencyMode::Limited(_)
        )
    }

    /// Render the tool call for display to the user.
    ///
    /// Generates a human-readable representation of the tool call, typically
    /// shown in logs or UI. The default implementation shows the tool name
    /// and JSON-formatted arguments.
    ///
    /// Override this method to provide custom formatting for better readability.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// use sage_core::tools::base::ToolError;
    /// use sage_core::tools::types::{ToolCall, ToolResult};
    /// use async_trait::async_trait;
    ///
    /// struct CustomDisplayTool;
    ///
    /// #[async_trait]
    /// impl Tool for CustomDisplayTool {
    ///     fn name(&self) -> &str { "custom" }
    ///     fn description(&self) -> &str { "Custom display" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     fn render_call(&self, call: &ToolCall) -> String {
    ///         format!("Executing {} with custom format", self.name())
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    /// ```
    fn render_call(&self, call: &ToolCall) -> String {
        format!(
            "{}({})",
            self.name(),
            serde_json::to_string(&call.arguments).unwrap_or_default()
        )
    }

    /// Render the tool result for display to the user.
    ///
    /// Generates a human-readable representation of the tool's execution result.
    /// The default implementation shows the output for successful executions
    /// and formats error messages for failures.
    ///
    /// Override this method to provide custom result formatting.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// use sage_core::tools::base::ToolError;
    /// use sage_core::tools::types::{ToolCall, ToolResult};
    /// use async_trait::async_trait;
    ///
    /// struct CustomResultTool;
    ///
    /// #[async_trait]
    /// impl Tool for CustomResultTool {
    ///     fn name(&self) -> &str { "custom" }
    ///     fn description(&self) -> &str { "Custom result display" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///
    ///     fn render_result(&self, result: &ToolResult) -> String {
    ///         if result.success {
    ///             format!("✓ Success: {}", result.output.as_deref().unwrap_or("done"))
    ///         } else {
    ///             format!("✗ Failed: {}", result.error.as_deref().unwrap_or("unknown"))
    ///         }
    ///     }
    ///
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    /// ```
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

    /// Execute the tool with timing and error handling.
    ///
    /// This method wraps `execute()` with automatic validation, timing
    /// measurement, and error conversion. It always returns a `ToolResult`,
    /// converting any errors into failed results.
    ///
    /// The execution flow:
    /// 1. Validates arguments using `validate()`
    /// 2. Executes the tool using `execute()`
    /// 3. Measures execution time
    /// 4. Converts errors to `ToolResult::error`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::{Tool, ToolSchema};
    /// use sage_core::tools::base::ToolError;
    /// use sage_core::tools::types::{ToolCall, ToolResult};
    /// use async_trait::async_trait;
    /// use std::collections::HashMap;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn description(&self) -> &str { "A tool" }
    ///     fn schema(&self) -> ToolSchema {
    ///         ToolSchema::new(self.name(), self.description(), vec![])
    ///     }
    ///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success(&call.id, self.name(), "done"))
    ///     }
    /// }
    ///
    /// # async fn example() {
    /// let tool = MyTool;
    /// let call = ToolCall {
    ///     id: "1".to_string(),
    ///     name: "my_tool".to_string(),
    ///     arguments: HashMap::new(),
    ///     call_id: None,
    /// };
    ///
    /// let result = tool.execute_with_timing(&call).await;
    /// assert!(result.success);
    /// assert!(result.execution_time_ms.is_some());
    /// # }
    /// ```
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

/// Macro to help implement the Tool trait.
///
/// This macro generates boilerplate implementations for:
/// - `new()` constructor
/// - `Default` trait
/// - Basic `Tool` trait methods (`name()` and `description()`)
///
/// You still need to implement:
/// - `schema()` - Define parameter schema
/// - `execute()` - Implement tool logic
/// - Optional trait methods (validation, permissions, etc.)
///
/// # Examples
///
/// ```ignore
/// use sage_core::impl_tool;
/// use sage_core::tools::Tool;
///
/// struct HelloTool;
///
/// // This macro implements new(), Default, and basic Tool trait (name, description)
/// impl_tool!(HelloTool, "hello", "Says hello");
///
/// // NOTE: You still need to implement schema() and execute() for Tool trait
/// // The macro only provides the name() and description() methods.
///
/// let tool = HelloTool::new();
/// assert_eq!(tool.name(), "hello");
/// assert_eq!(tool.description(), "Says hello");
/// ```
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

/// Helper trait for tools that need access to the file system.
///
/// Provides common functionality for file-based tools including path resolution
/// and security checks to prevent path traversal attacks.
///
/// # Security
///
/// The `is_safe_path()` method prevents malicious paths from escaping the
/// working directory using techniques like `../../../etc/passwd` or symlinks.
///
/// # Examples
///
/// ```no_run
/// use sage_core::tools::{Tool, ToolSchema};
/// use sage_core::tools::base::{FileSystemTool, ToolError};
/// use sage_core::tools::types::{ToolCall, ToolResult};
/// use async_trait::async_trait;
/// use std::path::{Path, PathBuf};
///
/// struct ReadTool {
///     working_dir: PathBuf,
/// }
///
/// #[async_trait]
/// impl Tool for ReadTool {
///     fn name(&self) -> &str { "read" }
///     fn description(&self) -> &str { "Read files" }
///     fn schema(&self) -> ToolSchema {
///         ToolSchema::new(self.name(), self.description(), vec![])
///     }
///
///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
///         let path_str = call.arguments.get("path")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| ToolError::InvalidArguments("path required".into()))?;
///
///         let path = self.resolve_path(path_str);
///
///         if !self.is_safe_path(&path) {
///             return Err(ToolError::PermissionDenied("Path outside working directory".into()));
///         }
///
///         // Read file...
///         Ok(ToolResult::success(&call.id, self.name(), "file contents"))
///     }
/// }
///
/// impl FileSystemTool for ReadTool {
///     fn working_directory(&self) -> &Path {
///         &self.working_dir
///     }
/// }
/// ```
pub trait FileSystemTool: Tool {
    /// Get the working directory for file operations.
    ///
    /// All file paths should be resolved relative to this directory.
    fn working_directory(&self) -> &std::path::Path;

    /// Resolve a relative path to an absolute path.
    ///
    /// If the path is already absolute, it is returned unchanged.
    /// Otherwise, it is joined with the working directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::base::FileSystemTool;
    /// # use sage_core::tools::{Tool, ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    /// # use std::path::{Path, PathBuf};
    ///
    /// # struct MyTool { working_dir: PathBuf }
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn description(&self) -> &str { "A tool" }
    /// #     fn schema(&self) -> ToolSchema { ToolSchema::new(self.name(), self.description(), vec![]) }
    /// #     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    /// #         Ok(ToolResult::success(&call.id, self.name(), "done"))
    /// #     }
    /// # }
    /// # impl FileSystemTool for MyTool {
    /// #     fn working_directory(&self) -> &Path { &self.working_dir }
    /// # }
    ///
    /// # fn example() {
    /// let tool = MyTool { working_dir: PathBuf::from("/home/user/project") };
    ///
    /// // Relative path gets joined with working dir
    /// let resolved = tool.resolve_path("src/main.rs");
    /// assert_eq!(resolved, PathBuf::from("/home/user/project/src/main.rs"));
    ///
    /// // Absolute path is unchanged
    /// let resolved = tool.resolve_path("/etc/hosts");
    /// assert_eq!(resolved, PathBuf::from("/etc/hosts"));
    /// # }
    /// ```
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

/// Helper trait for tools that execute commands.
///
/// Provides functionality for command execution with security controls
/// including command whitelisting and environment management.
///
/// # Security
///
/// The `is_command_allowed()` method restricts which commands can be executed
/// to prevent malicious command injection.
///
/// # Examples
///
/// ```no_run
/// use sage_core::tools::{Tool, ToolSchema};
/// use sage_core::tools::base::{CommandTool, ToolError};
/// use sage_core::tools::types::{ToolCall, ToolResult};
/// use async_trait::async_trait;
/// use std::path::{Path, PathBuf};
/// use std::collections::HashMap;
///
/// struct ShellTool {
///     working_dir: PathBuf,
/// }
///
/// #[async_trait]
/// impl Tool for ShellTool {
///     fn name(&self) -> &str { "shell" }
///     fn description(&self) -> &str { "Execute shell commands" }
///     fn schema(&self) -> ToolSchema {
///         ToolSchema::new(self.name(), self.description(), vec![])
///     }
///
///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
///         let command = call.arguments.get("command")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| ToolError::InvalidArguments("command required".into()))?;
///
///         if !self.is_command_allowed(command) {
///             return Err(ToolError::PermissionDenied("Command not allowed".into()));
///         }
///
///         // Execute command...
///         Ok(ToolResult::success(&call.id, self.name(), "output"))
///     }
/// }
///
/// impl CommandTool for ShellTool {
///     fn allowed_commands(&self) -> Vec<&str> {
///         vec!["ls", "cat", "grep"]  // Only allow safe commands
///     }
///
///     fn command_working_directory(&self) -> &Path {
///         &self.working_dir
///     }
/// }
/// ```
pub trait CommandTool: Tool {
    /// Get the allowed commands for this tool.
    ///
    /// Return an empty vector to allow all commands (not recommended).
    /// Return a list of command names to whitelist specific commands.
    fn allowed_commands(&self) -> Vec<&str>;

    /// Check if a command is allowed to execute.
    ///
    /// Compares the command against the whitelist from `allowed_commands()`.
    /// If the whitelist is empty, all commands are allowed.
    ///
    /// # Arguments
    ///
    /// * `command` - The command string to check
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::base::CommandTool;
    /// # use sage_core::tools::{Tool, ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    /// # use std::path::{Path, PathBuf};
    ///
    /// # struct MyTool { working_dir: PathBuf }
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn description(&self) -> &str { "A tool" }
    /// #     fn schema(&self) -> ToolSchema { ToolSchema::new(self.name(), self.description(), vec![]) }
    /// #     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    /// #         Ok(ToolResult::success(&call.id, self.name(), "done"))
    /// #     }
    /// # }
    /// # impl CommandTool for MyTool {
    /// #     fn allowed_commands(&self) -> Vec<&str> { vec!["git", "npm"] }
    /// #     fn command_working_directory(&self) -> &Path { &self.working_dir }
    /// # }
    ///
    /// # fn example() {
    /// let tool = MyTool { working_dir: PathBuf::from(".") };
    ///
    /// assert!(tool.is_command_allowed("git status"));
    /// assert!(tool.is_command_allowed("npm install"));
    /// assert!(!tool.is_command_allowed("rm -rf /"));
    /// # }
    /// ```
    fn is_command_allowed(&self, command: &str) -> bool {
        let allowed = self.allowed_commands();
        if allowed.is_empty() {
            return true; // No restrictions
        }

        allowed.iter().any(|&allowed_cmd| {
            // Match exact command or command followed by space (subcommand)
            command == allowed_cmd || command.starts_with(&format!("{} ", allowed_cmd))
        })
    }

    /// Get the working directory for command execution.
    ///
    /// Commands will be executed in this directory.
    fn command_working_directory(&self) -> &std::path::Path;

    /// Get environment variables for command execution.
    ///
    /// These variables will be added to the command's environment.
    /// The default implementation returns an empty map.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::base::CommandTool;
    /// # use sage_core::tools::{Tool, ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    /// # use std::path::{Path, PathBuf};
    /// use std::collections::HashMap;
    ///
    /// # struct MyTool { working_dir: PathBuf }
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn description(&self) -> &str { "A tool" }
    /// #     fn schema(&self) -> ToolSchema { ToolSchema::new(self.name(), self.description(), vec![]) }
    /// #     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    /// #         Ok(ToolResult::success(&call.id, self.name(), "done"))
    /// #     }
    /// # }
    /// impl CommandTool for MyTool {
    ///     fn allowed_commands(&self) -> Vec<&str> { vec![] }
    ///     fn command_working_directory(&self) -> &Path { &self.working_dir }
    ///
    ///     fn command_environment(&self) -> HashMap<String, String> {
    ///         let mut env = HashMap::new();
    ///         env.insert("NODE_ENV".to_string(), "production".to_string());
    ///         env.insert("DEBUG".to_string(), "false".to_string());
    ///         env
    ///     }
    /// }
    /// ```
    fn command_environment(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Mock tool for testing
    struct MockTool {
        name: String,
        description: String,
        working_dir: PathBuf,
    }

    impl MockTool {
        fn new(working_dir: PathBuf) -> Self {
            Self {
                name: "mock_tool".to_string(),
                description: "A mock tool for testing".to_string(),
                working_dir,
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("test-id", self.name(), "success"))
        }
    }

    impl FileSystemTool for MockTool {
        fn working_directory(&self) -> &std::path::Path {
            &self.working_dir
        }
    }

    struct MockCommandTool {
        allowed: Vec<String>,
        working_dir: PathBuf,
    }

    impl MockCommandTool {
        fn new(allowed: Vec<String>, working_dir: PathBuf) -> Self {
            Self {
                allowed,
                working_dir,
            }
        }
    }

    #[async_trait]
    impl Tool for MockCommandTool {
        fn name(&self) -> &str {
            "mock_command_tool"
        }

        fn description(&self) -> &str {
            "A mock command tool for testing"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("test-id", self.name(), "success"))
        }
    }

    impl CommandTool for MockCommandTool {
        fn allowed_commands(&self) -> Vec<&str> {
            self.allowed.iter().map(|s| s.as_str()).collect()
        }

        fn command_working_directory(&self) -> &std::path::Path {
            &self.working_dir
        }
    }

    #[test]
    fn test_tool_error_conversions() {
        // Test NotFound error
        let err = ToolError::NotFound("test_tool".to_string());
        let sage_err: SageError = err.into();
        assert!(sage_err.to_string().contains("Tool not found"));

        // Test InvalidArguments error
        let err = ToolError::InvalidArguments("invalid arg".to_string());
        let sage_err: SageError = err.into();
        assert!(sage_err.to_string().contains("invalid arg"));

        // Test Timeout error
        let err = ToolError::Timeout;
        let sage_err: SageError = err.into();
        assert!(sage_err.to_string().contains("timeout"));
    }

    #[test]
    fn test_concurrency_mode_equality() {
        assert_eq!(ConcurrencyMode::Parallel, ConcurrencyMode::Parallel);
        assert_eq!(ConcurrencyMode::Sequential, ConcurrencyMode::Sequential);
        assert_eq!(ConcurrencyMode::Limited(5), ConcurrencyMode::Limited(5));
        assert_ne!(ConcurrencyMode::Limited(5), ConcurrencyMode::Limited(10));
        assert_eq!(
            ConcurrencyMode::ExclusiveByType,
            ConcurrencyMode::ExclusiveByType
        );
    }

    #[test]
    fn test_concurrency_mode_default() {
        let mode: ConcurrencyMode = Default::default();
        assert_eq!(mode, ConcurrencyMode::Parallel);
    }

    #[test]
    fn test_filesystem_tool_resolve_absolute_path() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir.clone());

        let absolute = temp_dir.join("test.txt");
        let resolved = tool.resolve_path(&absolute.to_string_lossy());
        assert_eq!(resolved, absolute);
    }

    #[test]
    fn test_filesystem_tool_resolve_relative_path() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir.clone());

        let resolved = tool.resolve_path("test.txt");
        assert_eq!(resolved, temp_dir.join("test.txt"));
    }

    #[test]
    fn test_filesystem_tool_is_safe_path_within_working_dir() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir.clone());

        // Create a test file within the temp directory
        let safe_path = temp_dir.join("safe_file.txt");
        assert!(tool.is_safe_path(&safe_path));
    }

    #[test]
    fn test_filesystem_tool_is_safe_path_traversal_attack() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir.clone());

        // Try to escape using parent directory
        let unsafe_path = temp_dir.join("../../../etc/passwd");
        // After canonicalization, this should be outside the working directory
        // Note: This test may behave differently on different systems
        // The key is that is_safe_path should prevent escaping the working directory
        let canonical_unsafe = unsafe_path.canonicalize();
        if let Ok(canon) = canonical_unsafe {
            // Only test if canonicalization succeeds
            if !canon.starts_with(&temp_dir) {
                assert!(!tool.is_safe_path(&unsafe_path));
            }
        }
    }

    #[test]
    fn test_command_tool_no_restrictions() {
        let temp_dir = std::env::temp_dir();
        let tool = MockCommandTool::new(vec![], temp_dir);

        // With empty allowed list, all commands should be allowed
        assert!(tool.is_command_allowed("ls"));
        assert!(tool.is_command_allowed("echo hello"));
        assert!(tool.is_command_allowed("rm -rf /"));
    }

    #[test]
    fn test_command_tool_with_restrictions() {
        let temp_dir = std::env::temp_dir();
        let tool = MockCommandTool::new(
            vec!["ls".to_string(), "cat".to_string(), "echo".to_string()],
            temp_dir,
        );

        // Allowed commands
        assert!(tool.is_command_allowed("ls"));
        assert!(tool.is_command_allowed("ls -la"));
        assert!(tool.is_command_allowed("cat file.txt"));
        assert!(tool.is_command_allowed("echo hello"));

        // Disallowed commands
        assert!(!tool.is_command_allowed("rm file.txt"));
        assert!(!tool.is_command_allowed("sudo su"));
        assert!(!tool.is_command_allowed("wget malicious.com"));
    }

    #[test]
    fn test_command_tool_prefix_matching() {
        let temp_dir = std::env::temp_dir();
        let tool = MockCommandTool::new(vec!["git".to_string()], temp_dir);

        // All git commands should be allowed
        assert!(tool.is_command_allowed("git status"));
        assert!(tool.is_command_allowed("git commit -m 'test'"));
        assert!(tool.is_command_allowed("git push origin main"));

        // Non-git commands should be disallowed
        assert!(!tool.is_command_allowed("ls"));
        assert!(!tool.is_command_allowed("github"));
    }

    #[tokio::test]
    async fn test_execute_with_timing_success() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir);

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "mock_tool".to_string(),
            arguments: std::collections::HashMap::new(),
            call_id: None,
        };

        let result = tool.execute_with_timing(&call).await;
        assert!(result.success);
        assert!(result.execution_time_ms.is_some());
        assert!(result.execution_time_ms.unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_execute_with_timing_validation_error() {
        struct ValidatingTool;

        #[async_trait]
        impl Tool for ValidatingTool {
            fn name(&self) -> &str {
                "validating_tool"
            }

            fn description(&self) -> &str {
                "A tool that validates"
            }

            fn schema(&self) -> ToolSchema {
                ToolSchema::new(self.name(), self.description(), vec![])
            }

            async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
                Ok(ToolResult::success("test-id", self.name(), "success"))
            }

            fn validate(&self, _call: &ToolCall) -> Result<(), ToolError> {
                Err(ToolError::ValidationFailed(
                    "Validation failed".to_string(),
                ))
            }
        }

        let tool = ValidatingTool;
        let call = ToolCall {
            id: "test-2".to_string(),
            name: "validating_tool".to_string(),
            arguments: std::collections::HashMap::new(),
            call_id: None,
        };

        let result = tool.execute_with_timing(&call).await;
        assert!(!result.success);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .contains("Validation failed"));
        assert!(result.execution_time_ms.is_some());
    }

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::NotFound("test_tool".to_string());
        assert_eq!(err.to_string(), "Tool not found: test_tool");

        let err = ToolError::InvalidArguments("bad arg".to_string());
        assert_eq!(err.to_string(), "Invalid arguments: bad arg");

        let err = ToolError::Timeout;
        assert_eq!(err.to_string(), "Tool execution timeout");

        let err = ToolError::Cancelled;
        assert_eq!(err.to_string(), "Tool execution cancelled");
    }

    #[test]
    fn test_supports_parallel_execution() {
        struct ParallelTool;

        #[async_trait]
        impl Tool for ParallelTool {
            fn name(&self) -> &str {
                "parallel_tool"
            }

            fn description(&self) -> &str {
                "A parallel tool"
            }

            fn schema(&self) -> ToolSchema {
                ToolSchema::new(self.name(), self.description(), vec![])
            }

            async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
                Ok(ToolResult::success("test-id", self.name(), "success"))
            }

            fn concurrency_mode(&self) -> ConcurrencyMode {
                ConcurrencyMode::Parallel
            }
        }

        struct SequentialTool;

        #[async_trait]
        impl Tool for SequentialTool {
            fn name(&self) -> &str {
                "sequential_tool"
            }

            fn description(&self) -> &str {
                "A sequential tool"
            }

            fn schema(&self) -> ToolSchema {
                ToolSchema::new(self.name(), self.description(), vec![])
            }

            async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
                Ok(ToolResult::success("test-id", self.name(), "success"))
            }

            fn concurrency_mode(&self) -> ConcurrencyMode {
                ConcurrencyMode::Sequential
            }
        }

        let parallel = ParallelTool;
        assert!(parallel.supports_parallel_execution());

        let sequential = SequentialTool;
        assert!(!sequential.supports_parallel_execution());
    }

    #[test]
    fn test_max_execution_duration() {
        struct CustomTimeTool;

        #[async_trait]
        impl Tool for CustomTimeTool {
            fn name(&self) -> &str {
                "custom_time_tool"
            }

            fn description(&self) -> &str {
                "A tool with custom timeout"
            }

            fn schema(&self) -> ToolSchema {
                ToolSchema::new(self.name(), self.description(), vec![])
            }

            async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
                Ok(ToolResult::success("test-id", self.name(), "success"))
            }

            fn max_execution_duration(&self) -> Option<Duration> {
                Some(Duration::from_secs(120))
            }
        }

        let tool = CustomTimeTool;
        assert_eq!(
            tool.max_execution_duration(),
            Some(Duration::from_secs(120))
        );
        assert_eq!(tool.max_execution_time(), Some(120));
    }

    #[test]
    fn test_is_read_only_default() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir);
        assert!(!tool.is_read_only());
    }

    #[test]
    fn test_render_call_and_result() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir);

        let mut args = std::collections::HashMap::new();
        args.insert("key".to_string(), serde_json::Value::String("value".to_string()));

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "mock_tool".to_string(),
            arguments: args,
            call_id: None,
        };

        let rendered = tool.render_call(&call);
        assert!(rendered.contains("mock_tool"));
        assert!(rendered.contains("key"));

        let success_result = ToolResult::success("test-id", "mock_tool", "Success!");
        let rendered = tool.render_result(&success_result);
        assert_eq!(rendered, "Success!");

        let error_result = ToolResult::error("test-id", "mock_tool", "Failed!");
        let rendered = tool.render_result(&error_result);
        assert!(rendered.contains("Error"));
        assert!(rendered.contains("Failed!"));
    }

    #[test]
    fn test_requires_user_interaction_default() {
        let temp_dir = std::env::temp_dir();
        let tool = MockTool::new(temp_dir);
        assert!(!tool.requires_user_interaction());
    }
}
