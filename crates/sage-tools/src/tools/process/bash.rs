//! Bash command execution tool

use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use sage_core::tools::base::{CommandTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use crate::utils::{maybe_truncate, check_command_efficiency, suggest_efficient_alternative};

/// Tool for executing bash commands
pub struct BashTool {
    working_directory: PathBuf,
    allowed_commands: Vec<String>,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            allowed_commands: Vec::new(), // Empty means all commands allowed
        }
    }

    /// Create a bash tool with specific working directory
    pub fn with_working_directory(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_directory: working_dir.into(),
            allowed_commands: Vec::new(),
        }
    }

    /// Create a bash tool with allowed commands
    pub fn with_allowed_commands(mut self, commands: impl Into<Vec<String>>) -> Self {
        self.allowed_commands = commands.into();
        self
    }

    /// Execute a command and return the result
    async fn execute_command(&self, command: &str) -> Result<ToolResult, ToolError> {
        // Check if command is allowed
        if !self.is_command_allowed(command) {
            return Err(ToolError::PermissionDenied(format!(
                "Command not allowed: {}",
                command
            )));
        }

        // Check for potentially inefficient commands and provide suggestions
        let mut warnings = Vec::new();
        if let Some(efficiency_warning) = check_command_efficiency(command) {
            warnings.push(efficiency_warning);
        }
        if let Some(alternative) = suggest_efficient_alternative(command) {
            warnings.push(format!("Suggested alternative: {}", alternative));
        }

        let start_time = std::time::Instant::now();

        // Execute the command
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&self.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in self.command_environment() {
            cmd.env(key, value);
        }

        let output = cmd.output().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to execute command: {}", e))
        })?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result_text = if stderr.is_empty() {
            maybe_truncate(&stdout)
        } else if stdout.is_empty() {
            maybe_truncate(&format!("STDERR:\n{}", stderr))
        } else {
            maybe_truncate(&format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr))
        };

        // Prepend warnings if any
        if !warnings.is_empty() {
            let warning_text = warnings.join("\n");
            result_text = format!("⚠️  EFFICIENCY WARNINGS:\n{}\n\n{}", warning_text, result_text);
        }

        Ok(ToolResult {
            call_id: "".to_string(), // Will be set by executor
            tool_name: self.name().to_string(),
            success: output.status.success(),
            output: Some(result_text),
            error: if output.status.success() {
                None
            } else {
                Some(format!("Command failed with exit code: {:?}", output.status.code()))
            },
            exit_code: output.status.code(),
            execution_time_ms: Some(execution_time),
            metadata: std::collections::HashMap::new(),
        })
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute bash commands in the shell. Use this tool to run system commands, file operations, and other shell tasks.

IMPORTANT: Avoid commands that produce excessive output:
- Use 'find . -name \"*.rs\" | head -20' instead of 'find . -name \"*.rs\"'
- Use 'ls -la | head -10' instead of 'ls -R'
- Use 'grep -n pattern file | head -10' for searches
- Always limit output with 'head', 'tail', or line count limits"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "command",
                "The bash command to execute",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call
            .get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        let mut result = self.execute_command(&command).await?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let command = call
            .get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        // Check for dangerous commands
        let dangerous_patterns = [
            "rm -rf /",
            ":(){ :|:& };:",  // Fork bomb
            "dd if=/dev/zero",
            "mkfs",
            "fdisk",
            "shutdown",
            "reboot",
            "halt",
        ];

        let command_lower = command.to_lowercase();
        for pattern in &dangerous_patterns {
            if command_lower.contains(pattern) {
                return Err(ToolError::PermissionDenied(format!(
                    "Dangerous command pattern detected: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(300) // 5 minutes
    }

    fn supports_parallel_execution(&self) -> bool {
        false // Commands might interfere with each other
    }
}

impl CommandTool for BashTool {
    fn allowed_commands(&self) -> Vec<&str> {
        self.allowed_commands.iter().map(|s| s.as_str()).collect()
    }

    fn command_working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }

    fn command_environment(&self) -> std::collections::HashMap<String, String> {
        let mut env = std::collections::HashMap::new();
        
        // Add some safe environment variables
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }
        
        if let Ok(home) = std::env::var("HOME") {
            env.insert("HOME".to_string(), home);
        }
        
        if let Ok(user) = std::env::var("USER") {
            env.insert("USER".to_string(), user);
        }
        
        env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_json::json;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_bash_tool_simple_command() {
        let tool = BashTool::new();
        let call = create_tool_call("test-1", "bash", json!({
            "command": "echo 'Hello, World!'"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_bash_tool_pwd_command() {
        let tool = BashTool::new();
        let call = create_tool_call("test-2", "bash", json!({
            "command": "pwd"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_invalid_command() {
        let tool = BashTool::new();
        let call = create_tool_call("test-3", "bash", json!({
            "command": "nonexistent_command_12345"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_with_working_directory() {
        let temp_dir = std::env::temp_dir();
        let tool = BashTool::with_working_directory(&temp_dir);
        let call = create_tool_call("test-4", "bash", json!({
            "command": "pwd"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains(&temp_dir.to_string_lossy().to_string()));
    }

    #[tokio::test]
    async fn test_bash_tool_missing_command() {
        let tool = BashTool::new();
        let call = create_tool_call("test-5", "bash", json!({}));

        let result = tool.execute(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn test_bash_tool_allowed_commands() {
        let tool = BashTool::new().with_allowed_commands(vec!["echo".to_string(), "pwd".to_string()]);

        // Test allowed command
        let call = create_tool_call("test-6a", "bash", json!({
            "command": "echo 'allowed'"
        }));
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Test disallowed command
        let call = create_tool_call("test-6b", "bash", json!({
            "command": "ls"
        }));
        let result = tool.execute(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Command not allowed"));
    }

    #[test]
    fn test_bash_tool_schema() {
        let tool = BashTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "bash");
        assert!(!schema.description.is_empty());
    }
}
