//! Command execution logic

use std::process::Stdio;
use std::sync::Arc;

use sage_core::tools::base::{CommandTool, ToolError};
use sage_core::tools::types::ToolResult;
use sage_core::tools::{BACKGROUND_REGISTRY, BackgroundShellTask};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::tools::utils::{check_command_efficiency, maybe_truncate, suggest_efficient_alternative};

use super::types::BashTool;

impl BashTool {
    /// Execute a command in the background
    #[instrument(skip(self), fields(command_preview = %command.chars().take(50).collect::<String>(), shell_id))]
    pub async fn execute_background(
        &self,
        command: &str,
        shell_id: Option<String>,
    ) -> Result<ToolResult, ToolError> {
        // Check if command is allowed
        let argv: Vec<String> = command.split_whitespace().map(String::from).collect();
        if !self.is_command_allowed(&argv) {
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

        Ok(ToolResult::success("", "bash", output)
            .with_metadata("shell_id", serde_json::Value::String(shell_id))
            .with_metadata("pid", serde_json::json!(pid))
            .with_execution_time(0))
    }

    /// Execute a command and return the result
    #[instrument(skip(self), fields(command_preview = %command.chars().take(50).collect::<String>()))]
    pub async fn execute_command(&self, command: &str) -> Result<ToolResult, ToolError> {
        // Check if command is allowed
        let argv: Vec<String> = command.split_whitespace().map(String::from).collect();
        if !self.is_command_allowed(&argv) {
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
            ToolError::ExecutionFailed(format!(
                "Failed to execute command in '{}': {}. Ensure bash is available and the working directory is accessible.",
                self.working_directory.display(),
                e
            ))
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
            result_text = format!(
                "⚠️  EFFICIENCY WARNINGS:\n{}\n\n{}",
                warning_text, result_text
            );
        }

        // Build result using standardized format
        let mut result = if output.status.success() {
            ToolResult::success("", "bash", result_text)
        } else {
            ToolResult::error(
                "",
                "bash",
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
}
