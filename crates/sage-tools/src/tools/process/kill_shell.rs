//! Kill/terminate background shell processes

use async_trait::async_trait;
use sage_core::tools::BACKGROUND_REGISTRY;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Tool for killing/terminating background shell processes
pub struct KillShellTool;

impl KillShellTool {
    /// Create a new KillShell tool
    pub fn new() -> Self {
        Self
    }

    /// Kill a background shell process by ID
    async fn kill_shell(&self, shell_id: &str) -> Result<ToolResult, ToolError> {
        if let Some(task) = BACKGROUND_REGISTRY.get(shell_id) {
            let pid = task.pid;

            task.kill().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to kill shell {}: {}", shell_id, e))
            })?;

            BACKGROUND_REGISTRY.remove(shell_id);

            return Ok(ToolResult::success(
                "",
                self.name(),
                format!(
                    "Successfully terminated background shell '{}' (PID {:?})",
                    shell_id, pid
                ),
            ));
        }

        Err(ToolError::NotFound(format!(
            "Background shell '{}' not found",
            shell_id
        )))
    }
}

impl Default for KillShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for KillShellTool {
    fn name(&self) -> &str {
        "KillShell"
    }

    fn description(&self) -> &str {
        "Kill/terminate a running background shell process by its ID.

Use this tool to stop background shell processes that were started and are no longer needed.
The tool will attempt graceful termination (SIGTERM) first, and force kill (SIGKILL) if needed.

Parameters:
- shell_id: The unique identifier of the background shell to terminate

Example: kill_shell(shell_id=\"shell_1\")"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "shell_id",
                "The unique identifier of the background shell to kill",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let shell_id = call.get_string("shell_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'shell_id' parameter".to_string())
        })?;

        if shell_id.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "shell_id cannot be empty".to_string(),
            ));
        }

        let start_time = std::time::Instant::now();
        let mut result = self.kill_shell(&shell_id).await?;

        result.call_id = call.id.clone();
        result.execution_time_ms =
            Some(u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX));

        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let shell_id = call.get_string("shell_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'shell_id' parameter".to_string())
        })?;

        if shell_id.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "shell_id cannot be empty".to_string(),
            ));
        }

        // Validate shell_id format (alphanumeric, underscores, hyphens only)
        if !shell_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ToolError::InvalidArguments(
                "shell_id must contain only alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(30)) // 30 seconds should be enough to kill a process
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Can kill multiple shells in parallel
    }

    fn is_read_only(&self) -> bool {
        false // Modifies system state
    }
}

#[cfg(test)]
#[path = "kill_shell_tests.rs"]
mod tests;
