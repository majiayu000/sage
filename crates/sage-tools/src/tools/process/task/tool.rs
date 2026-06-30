//! TaskTool implementation

use async_trait::async_trait;
use sage_core::agent::subagent::SubAgentGraph;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::permission::ToolContext;
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use std::sync::Arc;

use super::executor::{
    execute_task_background, execute_task_background_with_graph, execute_task_sync,
};
use super::schema::{task_tool_description, task_tool_schema};
use super::types::{GLOBAL_TASK_REGISTRY, TaskRegistry};

/// Task tool for spawning subagents
pub struct TaskTool {
    registry: Arc<TaskRegistry>,
    subagent_graph: Option<Arc<SubAgentGraph>>,
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
            subagent_graph: None,
        }
    }

    pub fn with_registry(registry: Arc<TaskRegistry>) -> Self {
        Self {
            registry,
            subagent_graph: None,
        }
    }

    pub fn with_registry_and_graph(
        registry: Arc<TaskRegistry>,
        subagent_graph: Arc<SubAgentGraph>,
    ) -> Self {
        Self {
            registry,
            subagent_graph: Some(subagent_graph),
        }
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
        self.execute_task(call, None).await
    }

    async fn execute_with_context(
        &self,
        call: &ToolCall,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        self.execute_task(call, Some(context)).await
    }

    fn include_in_subagent_runner(&self) -> bool {
        self.subagent_graph.is_none()
    }

    fn subagent_runner_tool(&self) -> Option<Arc<dyn Tool>> {
        self.subagent_graph
            .as_ref()
            .map(|_| Arc::new(TaskTool::with_registry(self.registry.clone())) as Arc<dyn Tool>)
    }
}

impl TaskTool {
    async fn execute_task(
        &self,
        call: &ToolCall,
        context: Option<&ToolContext>,
    ) -> Result<ToolResult, ToolError> {
        // Check if background execution is requested
        let run_in_background = call
            .arguments
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if run_in_background {
            if let (Some(graph), Some(context)) = (&self.subagent_graph, context) {
                return execute_task_background_with_graph(
                    call,
                    self.registry.clone(),
                    graph.clone(),
                    context,
                )
                .await
                .map_err(|e| ToolError::InvalidArguments(e.to_string()));
            }

            if self.subagent_graph.is_some() && context.is_none() {
                if let Some(resume) = call
                    .arguments
                    .get("resume")
                    .and_then(|value| value.as_str())
                {
                    return Err(ToolError::InvalidArguments(format!(
                        "Task resume '{resume}' requires execution context for graph-backed Task tools"
                    )));
                }
            }

            execute_task_background(call, self.registry.clone(), context)
                .await
                .map_err(|e| ToolError::InvalidArguments(e.to_string()))
        } else {
            execute_task_sync(
                call,
                self.registry.clone(),
                self.subagent_graph.clone(),
                context,
            )
            .await
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))
        }
    }
}
