//! TaskOutput tool for retrieving output from background shell tasks
//!
//! This tool allows retrieving stdout/stderr from background tasks that
//! were started with `run_in_background=true` in the Bash tool.

use async_trait::async_trait;
use sage_core::tools::BACKGROUND_REGISTRY;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Tool for retrieving output from background shell tasks
pub struct TaskOutputTool;

impl TaskOutputTool {
    /// Create a new TaskOutput tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskOutputTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }

    fn description(&self) -> &str {
        r#"Retrieve output from a background shell task.

Returns accumulated stdout and stderr from a background process started with
run_in_background=true. Can retrieve incremental output or all output.

Parameters:
- shell_id: The ID of the background shell task (e.g., "shell_1")
- incremental: If true, only return output since last read (default: false)
- block: If true, wait for task completion (default: false)
- timeout: Max wait time in ms when blocking (default: 30000, max: 600000)

Example: task_output(shell_id="shell_1", incremental=false)"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "shell_id",
                    "The ID of the background shell task to get output from",
                ),
                ToolParameter::boolean(
                    "incremental",
                    "If true, only return output since last read (default: false)",
                )
                .optional()
                .with_default(false),
                ToolParameter::boolean(
                    "block",
                    "If true, wait for task completion (default: false)",
                )
                .optional()
                .with_default(false),
                ToolParameter::number(
                    "timeout",
                    "Max wait time in ms when blocking (default: 30000, max: 600000)",
                )
                .optional()
                .with_default(30000.0),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();

        let shell_id = call.get_string("shell_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'shell_id' parameter".to_string())
        })?;

        let incremental = call.get_bool("incremental").unwrap_or(false);
        let block = call.get_bool("block").unwrap_or(false);
        let timeout_raw = call.get_number("timeout").unwrap_or(30000.0);
        let timeout_ms = if timeout_raw.is_finite() && timeout_raw >= 0.0 {
            (timeout_raw as u64).min(600000)
        } else {
            30000u64
        };

        // Get task from registry
        let task = BACKGROUND_REGISTRY.get(&shell_id).ok_or_else(|| {
            ToolError::NotFound(format!("Background shell '{}' not found", shell_id))
        })?;

        // If blocking, wait for completion
        if block {
            let timeout = std::time::Duration::from_millis(timeout_ms);
            let start = std::time::Instant::now();

            while task.is_running().await {
                if start.elapsed() >= timeout {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        // Get status
        let status = task.status().await;

        // Get output
        let (stdout, stderr) = if incremental {
            task.get_incremental_output().await
        } else {
            task.get_output().await
        };

        // Format result
        let output = format!(
            "{}\nStatus: {}\n\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
            task.format_info(),
            status,
            if stdout.is_empty() {
                "(empty)"
            } else {
                &stdout
            },
            if stderr.is_empty() {
                "(empty)"
            } else {
                &stderr
            },
        );

        let mut result = ToolResult::success(&call.id, self.name(), output);
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

        // Validate shell_id format
        if !shell_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ToolError::InvalidArguments(
                "shell_id must contain only alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            ));
        }

        // Validate timeout if provided
        if let Some(timeout) = call.get_number("timeout") {
            if timeout < 0.0 {
                return Err(ToolError::InvalidArguments(
                    "timeout must be non-negative".to_string(),
                ));
            }
            if timeout > 600000.0 {
                return Err(ToolError::InvalidArguments(
                    "timeout cannot exceed 600000ms (10 minutes)".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(600)) // 10 minutes max (when blocking)
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Reading output is safe in parallel
    }

    fn is_read_only(&self) -> bool {
        true // Only reads output, doesn't modify state
    }
}

#[cfg(test)]
#[path = "task_output_tests.rs"]
mod tests;
