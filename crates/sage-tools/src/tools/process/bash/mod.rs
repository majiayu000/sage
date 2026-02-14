//! Bash command execution tool

mod execution;
mod security;
mod types;

pub use security::{
    requires_user_confirmation, validate_command_comprehensive, validate_command_security,
    validate_command_with_strictness,
};
pub use types::BashTool;

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use tracing::instrument;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> &str {
        r#"Executes a given bash command in a persistent shell session with optional timeout, ensuring proper handling and security measures.

IMPORTANT: This tool is for terminal operations like git, npm, docker, etc. DO NOT use it for file operations (reading, writing, editing, searching, finding files) - use the specialized tools for this instead.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use `ls` to verify the parent directory exists and is the correct location
   - For example, before running "mkdir foo/bar", first use `ls foo` to check that "foo" exists and is the intended parent directory

2. Command Execution:
   - Always quote file paths that contain spaces with double quotes (e.g., cd "path with spaces/file.txt")
   - After ensuring proper quoting, execute the command.
   - Capture the output of the command.

Usage notes:
  - The command argument is required.
  - You can optionally run commands in the background using run_in_background=true. Use task_output(shell_id) to retrieve output later.
  - If the output exceeds 30000 characters, output will be truncated.

  - Avoid using Bash with the `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands, unless explicitly instructed. Instead, always prefer using the dedicated tools:
    - File search: Use Glob (NOT find or ls)
    - Content search: Use Grep (NOT grep or rg)
    - Read files: Use Read (NOT cat/head/tail)
    - Edit files: Use Edit (NOT sed/awk)
    - Write files: Use Write (NOT echo >/cat <<EOF)
    - Communication: Output text directly (NOT echo/printf)
  - When issuing multiple commands:
    - If the commands are independent and can run in parallel, make multiple Bash tool calls in a single message.
    - If the commands depend on each other and must run sequentially, use a single Bash call with '&&' to chain them together (e.g., `git add . && git commit -m "message" && git push`).
    - Use ';' only when you need to run commands sequentially but don't care if earlier commands fail
    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)
  - Try to maintain your current working directory throughout the session by using absolute paths and avoiding usage of `cd`. You may use `cd` if the User explicitly requests it.
    <good-example>
    pytest /foo/bar/tests
    </good-example>
    <bad-example>
    cd /foo/bar && pytest tests
    </bad-example>"#
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

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(300)) // 5 minutes
    }

    fn supports_parallel_execution(&self) -> bool {
        false // Commands might interfere with each other
    }
}

#[cfg(test)]
mod bash_tests;
