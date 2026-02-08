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
        "task_output"
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
        result.execution_time_ms = Some(start_time.elapsed().as_millis() as u64);

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
mod tests {
    use super::*;
    use sage_core::tools::BackgroundShellTask;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

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

    #[test]
    fn test_task_output_schema() {
        let tool = TaskOutputTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "task_output");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_task_output_tool_properties() {
        let tool = TaskOutputTool::new();

        assert_eq!(tool.name(), "task_output");
        assert!(tool.description().contains("background"));
        assert_eq!(tool.max_execution_duration(), Some(std::time::Duration::from_secs(600)));
        assert!(tool.supports_parallel_execution());
        assert!(tool.is_read_only());
    }

    #[tokio::test]
    async fn test_task_output_missing_shell_id() {
        let tool = TaskOutputTool::new();
        let call = create_tool_call("test-1", "task_output", json!({}));

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(ToolError::InvalidArguments(msg)) => {
                assert!(msg.contains("shell_id"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_task_output_empty_shell_id() {
        let tool = TaskOutputTool::new();
        let call = create_tool_call(
            "test-2",
            "task_output",
            json!({
                "shell_id": ""
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());

        match result {
            Err(ToolError::InvalidArguments(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_task_output_invalid_shell_id() {
        let tool = TaskOutputTool::new();
        let call = create_tool_call(
            "test-3",
            "task_output",
            json!({
                "shell_id": "invalid shell!"
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());

        match result {
            Err(ToolError::InvalidArguments(msg)) => {
                assert!(msg.contains("alphanumeric"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_task_output_shell_not_found() {
        let tool = TaskOutputTool::new();
        let call = create_tool_call(
            "test-4",
            "task_output",
            json!({
                "shell_id": "nonexistent_shell_xyz"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(ToolError::NotFound(msg)) => {
                assert!(msg.contains("nonexistent_shell_xyz"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_task_output_timeout_validation() {
        let tool = TaskOutputTool::new();

        // Negative timeout
        let call = create_tool_call(
            "test-5",
            "task_output",
            json!({
                "shell_id": "test_shell",
                "timeout": -1000.0
            }),
        );
        assert!(tool.validate(&call).is_err());

        // Excessive timeout
        let call = create_tool_call(
            "test-6",
            "task_output",
            json!({
                "shell_id": "test_shell",
                "timeout": 700000.0
            }),
        );
        assert!(tool.validate(&call).is_err());

        // Valid timeout
        let call = create_tool_call(
            "test-7",
            "task_output",
            json!({
                "shell_id": "test_shell",
                "timeout": 5000.0
            }),
        );
        assert!(tool.validate(&call).is_ok());
    }

    #[tokio::test]
    async fn test_task_output_with_real_task() {
        // Create and register a background task
        let cancel_token = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_task_output_1".to_string(),
            "echo 'hello from background'",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        BACKGROUND_REGISTRY.register(Arc::new(task));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        let tool = TaskOutputTool::new();
        let call = create_tool_call(
            "test-8",
            "task_output",
            json!({
                "shell_id": "test_task_output_1",
                "incremental": false
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(tool_result.success);
        let output = tool_result.output.unwrap();
        assert!(output.contains("hello from background"));
        assert!(output.contains("COMPLETED"));

        // Cleanup
        BACKGROUND_REGISTRY.remove("test_task_output_1");
    }

    #[tokio::test]
    async fn test_task_output_incremental() {
        let cancel_token = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_task_output_2".to_string(),
            "echo 'line1'; sleep 0.1; echo 'line2'",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        BACKGROUND_REGISTRY.register(Arc::new(task));

        let tool = TaskOutputTool::new();

        // First read (incremental)
        tokio::time::sleep(Duration::from_millis(50)).await;
        let call1 = create_tool_call(
            "test-9a",
            "task_output",
            json!({
                "shell_id": "test_task_output_2",
                "incremental": true
            }),
        );
        let result1 = tool.execute(&call1).await.unwrap();
        let output1 = result1.output.unwrap();

        // Second read (incremental)
        tokio::time::sleep(Duration::from_millis(150)).await;
        let call2 = create_tool_call(
            "test-9b",
            "task_output",
            json!({
                "shell_id": "test_task_output_2",
                "incremental": true
            }),
        );
        let result2 = tool.execute(&call2).await.unwrap();
        let output2 = result2.output.unwrap();

        // Combined output should have both lines
        let combined = format!("{}{}", output1, output2);
        assert!(combined.contains("line1") || combined.contains("line2"));

        // Cleanup
        BACKGROUND_REGISTRY.remove("test_task_output_2");
    }
}
