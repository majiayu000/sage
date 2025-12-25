//! TaskTool implementation

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use std::sync::Arc;

use super::executor::{execute_task_background, execute_task_sync};
use super::schema::{task_tool_description, task_tool_schema};
use super::types::{TaskRegistry, GLOBAL_TASK_REGISTRY};

/// Task tool for spawning subagents
pub struct TaskTool {
    registry: Arc<TaskRegistry>,
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskTool {
    pub fn new() -> Self {
        Self {
            registry: GLOBAL_TASK_REGISTRY.clone(),
        }
    }

    pub fn with_registry(registry: Arc<TaskRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "Task"
    }

    fn description(&self) -> &str {
        task_tool_description()
    }

    fn schema(&self) -> ToolSchema {
        task_tool_schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Check if background execution is requested
        let run_in_background = call
            .arguments
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if run_in_background {
            execute_task_background(call, self.registry.clone())
                .map_err(|e| ToolError::InvalidArguments(e))
        } else {
            execute_task_sync(call, self.registry.clone())
                .await
                .map_err(|e| ToolError::InvalidArguments(e))
        }
    }
}
