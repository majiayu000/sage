//! Agent lifecycle tool for graph-backed sub-agent tasks.

use async_trait::async_trait;
use sage_core::agent::subagent::{
    AgentGraphDepth, AgentGraphListQuery, AgentPath, ChildAgentSummary, SubAgentGraph,
};
use sage_core::thread_store::ThreadStatus;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_TIMEOUT_MS: f64 = 600_000.0;

/// Read-only lifecycle operations for graph-backed sub-agent tasks.
pub struct AgentLifecycleTool {
    subagent_graph: Option<Arc<SubAgentGraph>>,
}

impl AgentLifecycleTool {
    pub fn new() -> Self {
        Self {
            subagent_graph: None,
        }
    }

    pub fn with_graph(subagent_graph: Arc<SubAgentGraph>) -> Self {
        Self {
            subagent_graph: Some(subagent_graph),
        }
    }
}

impl Default for AgentLifecycleTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AgentLifecycleTool {
    fn name(&self) -> &str {
        "AgentLifecycle"
    }

    fn description(&self) -> &str {
        r#"List and wait for graph-backed sub-agent tasks.

Operations:
- list: list direct child agents or descendants for a parent thread.
- wait: wait for an agent_path to reach a terminal status.

This tool requires a runtime ThreadStore-backed SubAgentGraph."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new_flexible(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["list", "wait"],
                        "description": "Lifecycle operation to perform."
                    },
                    "parent_thread_id": {
                        "type": "string",
                        "description": "Parent thread id for operation=list."
                    },
                    "depth": {
                        "type": "string",
                        "enum": ["direct", "descendants"],
                        "description": "Child traversal depth for operation=list.",
                        "default": "direct"
                    },
                    "include_archived": {
                        "type": "boolean",
                        "description": "Include archived parent/children for operation=list.",
                        "default": false
                    },
                    "agent_path": {
                        "type": "string",
                        "description": "Stable agent path for operation=wait, for example agent://task_..."
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Max wait time in ms for operation=wait, default 30000, max 600000.",
                        "default": 30000
                    }
                },
                "required": ["operation"]
            }),
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        self.validate(call)?;
        let graph = self.subagent_graph.as_ref().ok_or_else(|| {
            ToolError::InvalidArguments(
                "AgentLifecycle requires a ThreadStore-backed SubAgentGraph".to_string(),
            )
        })?;
        let start_time = Instant::now();

        match operation(call)?.as_str() {
            "list" => self.execute_list(call, graph, start_time).await,
            "wait" => self.execute_wait(call, graph, start_time).await,
            _ => unreachable!("operation is validated"),
        }
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        match operation(call)?.as_str() {
            "list" => {
                let parent_thread_id = required_string(call, "parent_thread_id")?;
                validate_non_empty("parent_thread_id", &parent_thread_id)?;
                if let Some(depth) = call.get_string("depth") {
                    parse_depth(&depth)?;
                }
            }
            "wait" => {
                let agent_path = required_string(call, "agent_path")?;
                AgentPath::from_raw_path(agent_path)
                    .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
                validate_timeout(call)?;
            }
            other => {
                return Err(ToolError::InvalidArguments(format!(
                    "unsupported operation '{other}'"
                )));
            }
        }
        Ok(())
    }

    fn max_execution_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(600))
    }

    fn supports_parallel_execution(&self) -> bool {
        true
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

impl AgentLifecycleTool {
    async fn execute_list(
        &self,
        call: &ToolCall,
        graph: &SubAgentGraph,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let parent_thread_id = required_string(call, "parent_thread_id")?;
        let depth = parse_depth(call.get_string("depth").as_deref().unwrap_or("direct"))?;
        let query = AgentGraphListQuery {
            depth,
            include_archived: call.get_bool("include_archived").unwrap_or(false),
        };
        let children = graph
            .list_children(&parent_thread_id, query)
            .await
            .map_err(|err| ToolError::ExecutionFailed(err.to_string()))?;

        let output = if children.is_empty() {
            format!("No sub-agents found for parent thread '{parent_thread_id}'.")
        } else {
            let lines = children
                .iter()
                .map(format_summary_line)
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Parent thread: {parent_thread_id}\nDepth: {}\n\n{lines}",
                depth_label(depth)
            )
        };

        Ok(timed_result(call, self.name(), output, start_time)
            .with_metadata("operation", json!("list"))
            .with_metadata("parent_thread_id", json!(parent_thread_id))
            .with_metadata("depth", json!(depth_label(depth)))
            .with_metadata("children", json!(summaries_to_json(&children))))
    }

    async fn execute_wait(
        &self,
        call: &ToolCall,
        graph: &SubAgentGraph,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let agent_path = AgentPath::from_raw_path(required_string(call, "agent_path")?)
            .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
        let timeout = timeout_duration(call.get_number("timeout").unwrap_or(30_000.0));

        loop {
            let summary = graph
                .read_child(&agent_path)
                .await
                .map_err(|err| ToolError::NotFound(err.to_string()))?;
            if is_terminal(summary.status) {
                let output = format!(
                    "Agent {} reached terminal status {}.",
                    summary.agent_path.as_path_str(),
                    summary.status.as_str()
                );
                return Ok(timed_result(call, self.name(), output, start_time)
                    .with_metadata("operation", json!("wait"))
                    .with_metadata("agent", summary_to_json(&summary))
                    .with_metadata("status", json!(summary.status.as_str())));
            }
            if start_time.elapsed() >= timeout {
                return Err(ToolError::Timeout);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

fn timed_result(
    call: &ToolCall,
    tool_name: &str,
    output: String,
    start_time: Instant,
) -> ToolResult {
    let mut result = ToolResult::success(&call.id, tool_name, output);
    result.execution_time_ms =
        Some(u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX));
    result
}

fn operation(call: &ToolCall) -> Result<String, ToolError> {
    required_string(call, "operation")
}

fn required_string(call: &ToolCall, name: &str) -> Result<String, ToolError> {
    call.get_string(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ToolError::InvalidArguments(format!("missing required '{name}'")))
}

fn validate_non_empty(name: &str, value: &str) -> Result<(), ToolError> {
    if value.trim().is_empty() {
        Err(ToolError::InvalidArguments(format!(
            "{name} cannot be empty"
        )))
    } else {
        Ok(())
    }
}

fn validate_timeout(call: &ToolCall) -> Result<(), ToolError> {
    if let Some(timeout) = call.get_number("timeout") {
        if !timeout.is_finite() || timeout < 0.0 {
            return Err(ToolError::InvalidArguments(
                "timeout must be a non-negative finite number".to_string(),
            ));
        }
        if timeout > MAX_TIMEOUT_MS {
            return Err(ToolError::InvalidArguments(
                "timeout cannot exceed 600000ms (10 minutes)".to_string(),
            ));
        }
    }
    Ok(())
}

fn timeout_duration(timeout_raw: f64) -> Duration {
    if timeout_raw.is_finite() && timeout_raw >= 0.0 {
        Duration::from_secs_f64(timeout_raw.min(MAX_TIMEOUT_MS) / 1000.0)
    } else {
        Duration::from_secs(30)
    }
}

fn parse_depth(raw: &str) -> Result<AgentGraphDepth, ToolError> {
    match raw {
        "direct" => Ok(AgentGraphDepth::Direct),
        "descendants" => Ok(AgentGraphDepth::Descendants),
        other => Err(ToolError::InvalidArguments(format!(
            "unsupported depth '{other}'"
        ))),
    }
}

fn depth_label(depth: AgentGraphDepth) -> &'static str {
    match depth {
        AgentGraphDepth::Direct => "direct",
        AgentGraphDepth::Descendants => "descendants",
    }
}

fn is_terminal(status: ThreadStatus) -> bool {
    matches!(
        status,
        ThreadStatus::Completed | ThreadStatus::Failed | ThreadStatus::Interrupted
    )
}

fn summaries_to_json(children: &[ChildAgentSummary]) -> Vec<Value> {
    children.iter().map(summary_to_json).collect()
}

fn summary_to_json(summary: &ChildAgentSummary) -> Value {
    json!({
        "agent_path": summary.agent_path.as_path_str(),
        "parent_thread_id": summary.parent_thread_id,
        "child_thread_id": summary.child_thread_id,
        "parent_turn_id": summary.parent_turn_id,
        "spawn_item_id": summary.spawn_item_id,
        "status": summary.status.as_str(),
        "archived": summary.archived,
        "title": summary.title,
        "created_at": summary.created_at,
        "updated_at": summary.updated_at,
    })
}

fn format_summary_line(summary: &ChildAgentSummary) -> String {
    format!(
        "- {} status={} parent={} spawn_item={}",
        summary.agent_path.as_path_str(),
        summary.status.as_str(),
        summary.parent_thread_id,
        summary.spawn_item_id
    )
}

#[cfg(test)]
#[path = "agent_lifecycle_tests.rs"]
mod tests;
