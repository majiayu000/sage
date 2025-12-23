//! Bash command execution tool

use crate::utils::{check_command_efficiency, maybe_truncate, suggest_efficient_alternative};
use async_trait::async_trait;
use sage_core::tools::base::{CommandTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use sage_core::tools::{BACKGROUND_REGISTRY, BackgroundShellTask};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

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

    /// Execute a command in the background
    #[instrument(skip(self), fields(command_preview = %command.chars().take(50).collect::<String>(), shell_id))]
    async fn execute_background(
        &self,
        command: &str,
        shell_id: Option<String>,
    ) -> Result<ToolResult, ToolError> {
        // Check if command is allowed
        if !self.is_command_allowed(command) {
            return Err(ToolError::PermissionDenied(format!(
                "Command not allowed: {}",
                command
            )));
        }

        // Generate or use provided shell ID
        let shell_id = shell_id.unwrap_or_else(|| BACKGROUND_REGISTRY.generate_shell_id());
        tracing::Span::current().record("shell_id", &shell_id);

        // Create cancellation token
        let cancel_token = CancellationToken::new();

        // Spawn background task
        let task = BackgroundShellTask::spawn(
            shell_id.clone(),
            command,
            &self.working_directory,
            cancel_token,
        )
        .await
        .map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to spawn background shell '{}' in '{}': {}. Verify the working directory exists and you have execute permissions.",
                shell_id, self.working_directory.display(), e
            ))
        })?;

        let pid = task.pid;

        // Register in global registry
        BACKGROUND_REGISTRY.register(Arc::new(task));

        let output = format!(
            "Background shell started with ID: '{}' (PID: {:?})\n\
            Command: {}\n\
            Working directory: {}\n\n\
            Use task_output(shell_id=\"{}\") to retrieve output.\n\
            Use kill_shell(shell_id=\"{}\") to terminate.",
            shell_id,
            pid,
            command,
            self.working_directory.display(),
            shell_id,
            shell_id
        );

        Ok(ToolResult::success("", self.name(), output)
            .with_metadata("shell_id", serde_json::Value::String(shell_id))
            .with_metadata("pid", serde_json::json!(pid))
            .with_execution_time(0))
    }

    /// Execute a command and return the result
    #[instrument(skip(self), fields(command_preview = %command.chars().take(50).collect::<String>()))]
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

        let output = cmd
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!(
                "Failed to execute command in '{}': {}. Ensure bash is available and the working directory is accessible.",
                self.working_directory.display(), e
            )))?;

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
            result_text = format!(
                "⚠️  EFFICIENCY WARNINGS:\n{}\n\n{}",
                warning_text, result_text
            );
        }

        // Build result using standardized format
        let mut result = if output.status.success() {
            ToolResult::success("", self.name(), result_text)
        } else {
            ToolResult::error(
                "",
                self.name(),
                format!(
                    "Command failed with exit code: {:?}\n\n{}",
                    output.status.code(),
                    result_text
                ),
            )
        };

        // Set additional fields
        result.exit_code = output.status.code();
        result.execution_time_ms = Some(execution_time);

        // Add metadata
        result = result
            .with_metadata("command", serde_json::Value::String(command.to_string()))
            .with_metadata(
                "working_directory",
                serde_json::Value::String(self.working_directory.display().to_string()),
            );

        Ok(result)
    }

    /// Validate command for security issues
    ///
    /// This checks for:
    /// - Dangerous command patterns (system destruction, privilege escalation)
    /// - Shell operators that could enable command injection
    /// - Command substitution attempts
    pub fn validate_command_security(command: &str) -> Result<(), ToolError> {
        let command_lower = command.to_lowercase();

        // Dangerous command patterns - system destruction
        let dangerous_commands = [
            "rm -rf /",
            "rm -rf /*",
            "rm -rf ~",
            ":(){ :|:& };:", // Fork bomb
            ":(){:|:&};:",   // Fork bomb variant (no spaces)
            "dd if=/dev/zero",
            "dd if=/dev/random",
            "mkfs",
            "fdisk",
            "parted",
            "shutdown",
            "reboot",
            "halt",
            "poweroff",
            "init 0",
            "init 6",
            "telinit 0",
            "> /dev/sda",
            "mv /* /dev/null",
            "chmod -r 000 /",
            "chown -r",
        ];

        for pattern in &dangerous_commands {
            if command_lower.contains(pattern) {
                return Err(ToolError::PermissionDenied(format!(
                    "Dangerous command pattern detected: {}",
                    pattern
                )));
            }
        }

        // Privilege escalation commands
        let privilege_commands = ["sudo ", "su ", "doas ", "pkexec "];
        for pattern in &privilege_commands {
            if command_lower.starts_with(pattern)
                || command_lower.contains(&format!(" {}", pattern.trim()))
            {
                return Err(ToolError::PermissionDenied(format!(
                    "Privilege escalation command not allowed: {}",
                    pattern.trim()
                )));
            }
        }

        // Check for command substitution which could bypass validation
        // These allow executing arbitrary commands within the main command
        let substitution_patterns = [
            "$(", // Modern command substitution
            "`",  // Legacy command substitution (backticks)
            "${", // Variable expansion with commands
        ];

        for pattern in &substitution_patterns {
            if command.contains(pattern) {
                return Err(ToolError::PermissionDenied(format!(
                    "Command substitution not allowed: {}",
                    pattern
                )));
            }
        }

        // Check for dangerous shell operators that enable command chaining
        // Note: We allow pipes (|) and redirects (>, <) as they are commonly needed
        // but block command separators that could run arbitrary commands
        let dangerous_operators = [
            ";",  // Command separator - runs multiple commands
            "&&", // Logical AND - runs second command if first succeeds
            "||", // Logical OR - runs second command if first fails
        ];

        for op in &dangerous_operators {
            if command.contains(op) {
                return Err(ToolError::PermissionDenied(format!(
                    "Command chaining operator not allowed: '{}'",
                    op
                )));
            }
        }

        // Check for process backgrounding which could escape control
        if command.trim().ends_with('&') && !command.trim().ends_with("&&") {
            return Err(ToolError::PermissionDenied(
                "Background process operator (&) not allowed at end of command".to_string(),
            ));
        }

        Ok(())
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
        r#"Execute bash commands in the shell. Use this tool to run system commands, file operations, and other shell tasks.

Parameters:
- command: The bash command to execute
- run_in_background: If true, run command in background (default: false)
- shell_id: Optional custom ID for background shell (auto-generated if not provided)

IMPORTANT: Avoid commands that produce excessive output:
- Use 'find . -name "*.rs" | head -20' instead of 'find . -name "*.rs"'
- Use 'ls -la | head -10' instead of 'ls -R'
- Use 'grep -n pattern file | head -10' for searches
- Always limit output with 'head', 'tail', or line count limits

Background mode:
When run_in_background=true, the command starts and returns immediately with a shell_id.
Use task_output(shell_id) to retrieve output and kill_shell(shell_id) to terminate."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("command", "The bash command to execute"),
                ToolParameter::boolean(
                    "run_in_background",
                    "If true, run command in background (default: false)",
                )
                .optional()
                .with_default(false),
                ToolParameter::optional_string(
                    "shell_id",
                    "Optional custom ID for background shell (auto-generated if not provided)",
                ),
            ],
        )
    }

    #[instrument(skip(self, call), fields(call_id = %call.id, run_in_background))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        let run_in_background = call.get_bool("run_in_background").unwrap_or(false);
        tracing::Span::current().record("run_in_background", run_in_background);

        let shell_id = call.get_string("shell_id");

        tracing::debug!(
            command_preview = %command.chars().take(100).collect::<String>(),
            "executing bash command"
        );

        let mut result = if run_in_background {
            self.execute_background(&command, shell_id).await?
        } else {
            self.execute_command(&command).await?
        };

        if result.success {
            tracing::info!("bash command completed successfully");
        } else {
            tracing::warn!("bash command failed");
        }

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        // Validate the command for security issues
        Self::validate_command_security(&command)?;

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
    use serde_json::json;
    use std::collections::HashMap;

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
        let call = create_tool_call(
            "test-1",
            "bash",
            json!({
                "command": "echo 'Hello, World!'"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_bash_tool_pwd_command() {
        let tool = BashTool::new();
        let call = create_tool_call(
            "test-2",
            "bash",
            json!({
                "command": "pwd"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_invalid_command() {
        let tool = BashTool::new();
        let call = create_tool_call(
            "test-3",
            "bash",
            json!({
                "command": "nonexistent_command_12345"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_with_working_directory() {
        let temp_dir = std::env::temp_dir();
        let tool = BashTool::with_working_directory(&temp_dir);
        let call = create_tool_call(
            "test-4",
            "bash",
            json!({
                "command": "pwd"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        // Just verify we got some output - temp dir paths may differ after canonicalization
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_missing_command() {
        let tool = BashTool::new();
        let call = create_tool_call("test-5", "bash", json!({}));

        // Implementation returns Err for missing parameters
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing") || err.to_string().contains("command"));
    }

    #[tokio::test]
    async fn test_bash_tool_allowed_commands() {
        let tool =
            BashTool::new().with_allowed_commands(vec!["echo".to_string(), "pwd".to_string()]);

        // Test allowed command
        let call = create_tool_call(
            "test-6a",
            "bash",
            json!({
                "command": "echo 'allowed'"
            }),
        );
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Test disallowed command - returns Err
        let call = create_tool_call(
            "test-6b",
            "bash",
            json!({
                "command": "ls"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not allowed") || err.to_string().contains("Command"));
    }

    #[test]
    fn test_bash_tool_schema() {
        let tool = BashTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "bash");
        assert!(!schema.description.is_empty());
    }

    // Security validation tests
    #[test]
    fn test_dangerous_commands_blocked() {
        // Test dangerous command patterns are blocked
        let dangerous_commands = vec![
            "rm -rf /",
            "rm -rf /*",
            ":(){ :|:& };:",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sda",
            "shutdown -h now",
            "reboot",
        ];

        for cmd in dangerous_commands {
            let result = BashTool::validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_privilege_escalation_blocked() {
        // Test privilege escalation commands are blocked
        let priv_commands = vec![
            "sudo rm -rf /tmp/test",
            "su - root",
            "doas ls",
            "pkexec /bin/bash",
        ];

        for cmd in priv_commands {
            let result = BashTool::validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_command_substitution_blocked() {
        // Test command substitution is blocked
        let subst_commands = vec!["echo $(whoami)", "echo `id`", "echo ${PATH}"];

        for cmd in subst_commands {
            let result = BashTool::validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_command_chaining_blocked() {
        // Test command chaining operators are blocked
        let chain_commands = vec![
            "echo hello; rm -rf /",
            "ls && cat /etc/passwd",
            "false || reboot",
        ];

        for cmd in chain_commands {
            let result = BashTool::validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_background_process_blocked() {
        // Test background process operator is blocked at end
        let result = BashTool::validate_command_security("sleep 9999 &");
        assert!(result.is_err(), "Background process should be blocked");
    }

    #[test]
    fn test_safe_commands_allowed() {
        // Test that safe commands are allowed
        let safe_commands = vec![
            "echo 'Hello, World!'",
            "ls -la",
            "pwd",
            "cat file.txt",
            "grep pattern file.txt",
            "find . -name '*.rs'",
            "head -n 10 file.txt",
            "tail -f log.txt",
            "wc -l file.txt",
        ];

        for cmd in safe_commands {
            let result = BashTool::validate_command_security(cmd);
            assert!(result.is_ok(), "Command should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_pipe_and_redirect_allowed() {
        // Test that pipes and redirects are still allowed (commonly needed)
        let pipe_commands = vec![
            "ls | head -10",
            "grep pattern file.txt | wc -l",
            "echo 'test' > output.txt",
            "cat file.txt >> output.txt",
        ];

        for cmd in pipe_commands {
            let result = BashTool::validate_command_security(cmd);
            assert!(result.is_ok(), "Command should be allowed: {}", cmd);
        }
    }
}
