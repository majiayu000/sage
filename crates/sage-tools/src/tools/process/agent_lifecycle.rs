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

use super::task::{GLOBAL_TASK_REGISTRY, TaskRegistry, TaskRequest, TaskStatus};

const MAX_TIMEOUT_MS: f64 = 600_000.0;
const EXECUTION_TIMEOUT_HEADROOM: Duration = Duration::from_secs(10);

/// Read-only lifecycle operations for graph-backed sub-agent tasks.
pub struct AgentLifecycleTool {
    task_registry: Arc<TaskRegistry>,
    subagent_graph: Option<Arc<SubAgentGraph>>,
}

impl AgentLifecycleTool {
    pub fn new() -> Self {
        Self {
            task_registry: GLOBAL_TASK_REGISTRY.clone(),
            subagent_graph: None,
        }
    }

    pub fn with_graph(subagent_graph: Arc<SubAgentGraph>) -> Self {
        Self {
            task_registry: GLOBAL_TASK_REGISTRY.clone(),
            subagent_graph: Some(subagent_graph),
        }
    }

    pub fn with_task_registry_and_graph(
        task_registry: Arc<TaskRegistry>,
        subagent_graph: Arc<SubAgentGraph>,
    ) -> Self {
        Self {
            task_registry,
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
        Some(timeout_duration(MAX_TIMEOUT_MS) + EXECUTION_TIMEOUT_HEADROOM)
    }

    fn include_in_subagent_runner(&self) -> bool {
        false
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
        self.task_registry.reconcile_finished_tasks().await;
        let children = graph
            .list_children(&parent_thread_id, query)
            .await
            .map_err(|err| ToolError::ExecutionFailed(err.to_string()))?;

        let output = if children.is_empty() {
            format!("No sub-agents found for parent thread '{parent_thread_id}'.")
        } else {
            let lines = children
                .iter()
                .map(|summary| {
                    let task = self.task_registry.get_task(&summary.child_thread_id);
                    format_summary_line(summary, task.as_ref())
                })
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
            .with_metadata(
                "children",
                json!(summaries_to_json(&children, &self.task_registry)),
            ))
    }

    async fn execute_wait(
        &self,
        call: &ToolCall,
        graph: &SubAgentGraph,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let agent_path = AgentPath::from_raw_path(required_string(call, "agent_path")?)
            .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
        let timeout_raw = call.get_number("timeout").unwrap_or(30_000.0);
        let timeout = timeout_duration(timeout_raw);

        loop {
            self.task_registry.reconcile_finished_tasks().await;
            let summary = graph
                .read_child(&agent_path)
                .await
                .map_err(|err| ToolError::NotFound(err.to_string()))?;
            let task = self.task_registry.get_task(&summary.child_thread_id);
            if terminal_status(summary.status, task.as_ref()).is_some() {
                let status = status_label(summary.status, task.as_ref());
                let output = format!(
                    "Agent {} reached terminal status {}.",
                    summary.agent_path.as_path_str(),
                    status
                );
                let result = timed_result(call, self.name(), output, start_time)
                    .with_metadata("operation", json!("wait"))
                    .with_metadata("agent", summary_to_json(&summary, task.as_ref()))
                    .with_metadata("status", json!(&status))
                    .with_metadata("graph_status", json!(summary.status.as_str()))
                    .with_metadata(
                        "task_status",
                        json!(task.as_ref().map(|task| task.status.to_string())),
                    );
                return if status == "completed" {
                    Ok(result)
                } else {
                    Ok(result
                        .into_error_result()
                        .with_metadata("error_code", json!(terminal_error_code(&status))))
                };
            }
            if start_time.elapsed() >= timeout {
                let last_status = status_label(summary.status, task.as_ref());
                return Ok(timed_result(
                    call,
                    self.name(),
                    format!(
                        "Timed out waiting for agent {} after {}ms; last status was {}.",
                        summary.agent_path.as_path_str(),
                        timeout_raw,
                        last_status
                    ),
                    start_time,
                )
                .into_error_result()
                .with_metadata("operation", json!("wait"))
                .with_metadata("error_code", json!("timeout"))
                .with_metadata("agent_path", json!(summary.agent_path.as_path_str()))
                .with_metadata("timeout_ms", json!(timeout_raw))
                .with_metadata("last_status", json!(last_status))
                .with_metadata("graph_status", json!(summary.status.as_str()))
                .with_metadata(
                    "task_status",
                    json!(task.as_ref().map(|task| task.status.to_string())),
                )
                .with_metadata("agent", summary_to_json(&summary, task.as_ref())));
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

trait AgentLifecycleResultExt {
    fn into_error_result(self) -> Self;
}

impl AgentLifecycleResultExt for ToolResult {
    fn into_error_result(mut self) -> Self {
        self.success = false;
        self.error = self.output.take();
        self.exit_code = Some(1);
        self
    }
}

fn terminal_status(graph_status: ThreadStatus, task: Option<&TaskRequest>) -> Option<String> {
    if let Some(task) = task {
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Failed) {
            return Some(task.status.to_string());
        }
    }
    if matches!(
        graph_status,
        ThreadStatus::Completed | ThreadStatus::Failed | ThreadStatus::Interrupted
    ) {
        Some(graph_status.as_str().to_string())
    } else {
        None
    }
}

fn status_label(graph_status: ThreadStatus, task: Option<&TaskRequest>) -> String {
    terminal_status(graph_status, task).unwrap_or_else(|| {
        task.map(|task| task.status.to_string())
            .unwrap_or_else(|| graph_status.as_str().to_string())
    })
}

fn terminal_error_code(status: &str) -> &'static str {
    match status {
        "failed" => "agent_failed",
        "interrupted" => "agent_interrupted",
        _ => "agent_terminal_error",
    }
}

fn summaries_to_json(children: &[ChildAgentSummary], registry: &TaskRegistry) -> Vec<Value> {
    children
        .iter()
        .map(|summary| {
            let task = registry.get_task(&summary.child_thread_id);
            summary_to_json(summary, task.as_ref())
        })
        .collect()
}

fn summary_to_json(summary: &ChildAgentSummary, task: Option<&TaskRequest>) -> Value {
    json!({
        "agent_path": summary.agent_path.as_path_str(),
        "parent_thread_id": summary.parent_thread_id,
        "child_thread_id": summary.child_thread_id,
        "parent_turn_id": summary.parent_turn_id,
        "spawn_item_id": summary.spawn_item_id,
        "status": status_label(summary.status, task),
        "graph_status": summary.status.as_str(),
        "archived": summary.archived,
        "title": summary.title,
        "created_at": summary.created_at,
        "updated_at": summary.updated_at,
        "task_status": task.map(|task| task.status.to_string()),
        "final_result": task.and_then(|task| {
            if task.status == TaskStatus::Completed {
                task.result.clone()
            } else {
                None
            }
        }),
        "error": task.and_then(|task| {
            if task.status == TaskStatus::Failed {
                task.result.clone()
            } else {
                None
            }
        }),
    })
}

fn format_summary_line(summary: &ChildAgentSummary, task: Option<&TaskRequest>) -> String {
    format!(
        "- {} status={} graph_status={} parent={} spawn_item={}",
        summary.agent_path.as_path_str(),
        status_label(summary.status, task),
        summary.status.as_str(),
        summary.parent_thread_id,
        summary.spawn_item_id
    )
}

#[cfg(test)]
#[path = "agent_lifecycle_tests.rs"]
mod tests;
