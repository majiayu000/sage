//! Task completion tool

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Tool for marking tasks as completed
pub struct TaskDoneTool;

impl TaskDoneTool {
    /// Create a new task done tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskDoneTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskDoneTool {
    fn name(&self) -> &str {
        "task_done"
    }

    fn description(&self) -> &str {
        "Use this tool when you have completed the assigned task. Provide a summary of what was accomplished."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "summary",
                    "A summary of what was accomplished in completing the task",
                ),
                ToolParameter::optional_string(
                    "details",
                    "Additional details about the task completion",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let summary = call
            .get_string("summary")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'summary' parameter".to_string()))?;

        if summary.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Summary cannot be empty".to_string(),
            ));
        }

        let details = call.get_string("details").unwrap_or_default();

        let mut completion_message = format!("✅ Task Completed Successfully!\n\nSummary: {}", summary);

        if !details.trim().is_empty() {
            completion_message.push_str(&format!("\n\nDetails:\n{}", details));
        }

        // Add timestamp
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        completion_message.push_str(&format!("\n\nCompleted at: {}", timestamp));

        Ok(ToolResult::success(
            &call.id,
            self.name(),
            completion_message,
        ))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let summary = call
            .get_string("summary")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'summary' parameter".to_string()))?;

        if summary.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Summary cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(5) // 5 seconds - this is a very lightweight operation
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Task completion doesn't interfere with other operations
    }
}
