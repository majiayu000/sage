//! Command execution helper trait for tools

use super::tool_trait::Tool;
use std::collections::HashMap;
use std::path::Path;

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
///         let argv = call.require_string_array("argv")?;
///
///         if !self.is_command_allowed(&argv) {
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
    /// * `argv` - The command argv to check (first element is the executable)
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
    /// assert!(tool.is_command_allowed(&vec!["git".into(), "status".into()]));
    /// assert!(tool.is_command_allowed(&vec!["npm".into(), "install".into()]));
    /// assert!(!tool.is_command_allowed(&vec!["rm".into(), "-rf".into(), "/".into()]));
    /// # }
    /// ```
    fn is_command_allowed(&self, argv: &[String]) -> bool {
        let allowed = self.allowed_commands();
        if allowed.is_empty() {
            return true; // No restrictions
        }

        let Some(command) = argv.first() else {
            return false;
        };

        let base = Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command.as_str());

        allowed.contains(&base)
    }

    /// Get the working directory for command execution.
    ///
    /// Commands will be executed in this directory.
    fn command_working_directory(&self) -> &Path;

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
    fn command_environment(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}
