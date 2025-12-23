//! Tool for viewing the current task list

use super::task_list::GLOBAL_TASK_LIST;
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

/// Tool for viewing the current task list
pub struct ViewTasklistTool;

impl Default for ViewTasklistTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewTasklistTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ViewTasklistTool {
    fn name(&self) -> &str {
        "view_tasklist"
    }

    fn description(&self) -> &str {
        "View the current task list for the conversation."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let tasklist = GLOBAL_TASK_LIST.view_tasklist();
        Ok(ToolResult::success(&tool_call.id, self.name(), tasklist))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }
}
