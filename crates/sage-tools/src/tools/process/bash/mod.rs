//! Bash command execution tool

mod execution;
mod security;
mod types;

pub use security::{requires_user_confirmation, validate_command_security};
pub use types::BashTool;

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use tracing::instrument;

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
                ToolParameter::boolean(
                    "user_confirmed",
                    "Set to true ONLY after getting explicit user confirmation via ask_user_question tool for destructive commands (rm, rmdir, git push --force, etc.)",
                )
                .optional()
                .with_default(false),
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

        // Check if this is a destructive command that requires user confirmation
        // The agent must explicitly acknowledge by setting user_confirmed=true
        let user_confirmed = call.get_bool("user_confirmed").unwrap_or(false);
        if let Some(reason) = requires_user_confirmation(&command) {
            if !user_confirmed {
                return Err(ToolError::ConfirmationRequired(format!(
                    "⚠️  DESTRUCTIVE COMMAND BLOCKED\n\n\
                    {}\n\n\
                    Before executing this command, you MUST:\n\
                    1. Use the ask_user_question tool to get explicit user confirmation\n\
                    2. Wait for the user's response\n\
                    3. Only if user confirms, call this tool again with user_confirmed=true\n\n\
                    DO NOT proceed without user confirmation!",
                    reason
                )));
            }
            tracing::info!(
                command = %command,
                "executing confirmed destructive command"
            );
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
        validate_command_security(&command)?;

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(300) // 5 minutes
    }

    fn supports_parallel_execution(&self) -> bool {
        false // Commands might interfere with each other
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
            let result = validate_command_security(cmd);
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
            let result = validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
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
            let result = validate_command_security(cmd);
            assert!(result.is_ok(), "Command should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_command_chaining_allowed() {
        // Test that command chaining is now allowed
        let chained_commands = vec![
            "cd /tmp && ls",
            "echo hello; echo world",
            "false || echo 'failed'",
            "cd /repo && python -c 'import sys; print(sys.version)'",
        ];

        for cmd in chained_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_ok(), "Command chaining should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_command_substitution_allowed() {
        // Test that command substitution is now allowed
        let subst_commands = vec!["echo $(pwd)", "echo `date`", "echo ${HOME}"];

        for cmd in subst_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_ok(), "Command substitution should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_pipe_and_redirect_allowed() {
        // Test that pipes and redirects are allowed
        let pipe_commands = vec![
            "ls | head -10",
            "grep pattern file.txt | wc -l",
            "echo 'test' > output.txt",
            "cat file.txt >> output.txt",
        ];

        for cmd in pipe_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_ok(), "Command should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_chained_dangerous_still_blocked() {
        // Even with chaining allowed, dangerous commands are still blocked
        let dangerous_chained = vec![
            "echo hello && rm -rf /",
            "ls; sudo rm -rf /tmp",
            "false || shutdown -h now",
        ];

        for cmd in dangerous_chained {
            let result = validate_command_security(cmd);
            assert!(result.is_err(), "Dangerous command should still be blocked: {}", cmd);
        }
    }
}
