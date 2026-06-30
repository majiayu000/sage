//! Graph-backed sub-agent messaging and interruption tool.
use async_trait::async_trait;
use sage_core::agent::subagent::{AgentPath, SubAgentGraph, SubAgentGraphError};
use sage_core::thread_store::{ThreadStatus, ThreadStoreError};
use sage_core::tools::base::{ConcurrencyMode, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

use super::task::{GLOBAL_TASK_REGISTRY, TaskRegistry, TaskRequest, TaskStatus};

const DEFAULT_INTERRUPT_REASON: &str = "interrupted by parent";

pub struct AgentMessagingTool {
    task_registry: Arc<TaskRegistry>,
    subagent_graph: Option<Arc<SubAgentGraph>>,
}

impl AgentMessagingTool {
    pub fn new() -> Self {
        Self {
            task_registry: GLOBAL_TASK_REGISTRY.clone(),
            subagent_graph: None,
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

impl Default for AgentMessagingTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AgentMessagingTool {
    fn name(&self) -> &str {
        "AgentMessaging"
    }

    fn description(&self) -> &str {
        r#"Send follow-up messages to graph-backed sub-agents or interrupt active sub-agents.
Requires a runtime ThreadStore-backed SubAgentGraph."#
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
                        "enum": ["follow_up", "interrupt"],
                        "description": "Messaging operation to perform."
                    },
                    "agent_path": {
                        "type": "string",
                        "description": "Stable agent path, for example agent://task_..."
                    },
                    "message": {
                        "type": "string",
                        "description": "Follow-up user message for operation=follow_up."
                    },
                    "reason": {
                        "type": "string",
                        "description": "Interrupt reason for operation=interrupt."
                    }
                },
                "required": ["operation", "agent_path"]
            }),
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        self.validate(call)?;
        let graph = self.subagent_graph.as_ref().ok_or_else(|| {
            ToolError::InvalidArguments(
                "AgentMessaging requires a ThreadStore-backed SubAgentGraph".to_string(),
            )
        })?;
        let start_time = Instant::now();
        match messaging_operation(call)?.as_str() {
            "follow_up" => self.execute_follow_up(call, graph, start_time).await,
            "interrupt" => self.execute_interrupt(call, graph, start_time).await,
            _ => unreachable!("operation is validated"),
        }
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        match messaging_operation(call)?.as_str() {
            "follow_up" => {
                parse_messaging_agent_path(required_messaging_string(call, "agent_path")?)?;
                validate_messaging_non_empty(
                    "message",
                    &required_messaging_string(call, "message")?,
                )?;
            }
            "interrupt" => {
                parse_messaging_agent_path(required_messaging_string(call, "agent_path")?)?;
                if let Some(reason) = call.get_string("reason") {
                    validate_messaging_non_empty("reason", &reason)?;
                }
            }
            other => {
                return Err(ToolError::InvalidArguments(format!(
                    "unsupported operation '{other}'"
                )));
            }
        }
        Ok(())
    }

    fn include_in_subagent_runner(&self) -> bool {
        false
    }

    fn supports_parallel_execution(&self) -> bool {
        false
    }

    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Sequential
    }

    fn is_read_only(&self) -> bool {
        false
    }
}

impl AgentMessagingTool {
    async fn execute_follow_up(
        &self,
        call: &ToolCall,
        graph: &Arc<SubAgentGraph>,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let agent_path =
            parse_messaging_agent_path(required_messaging_string(call, "agent_path")?)?;
        let message = required_messaging_string(call, "message")?;
        self.task_registry.reconcile_finished_tasks().await;
        let summary = match graph.read_child(&agent_path).await {
            Ok(summary) => summary,
            Err(err) => {
                return Ok(graph_error_result(
                    call,
                    self.name(),
                    "follow_up",
                    agent_path.as_path_str(),
                    err,
                    start_time,
                ));
            }
        };
        if is_graph_terminal(summary.status) {
            return Ok(messaging_invalid_state_result(
                call,
                self.name(),
                "follow_up",
                agent_path.as_path_str(),
                summary.status.as_str().to_string(),
                start_time,
            ));
        }
        if !self
            .task_registry
            .get_task(&summary.child_thread_id)
            .is_some_and(|task| is_task_active(&task))
        {
            return Ok(messaging_unsupported_result(
                call,
                self.name(),
                agent_path.as_path_str(),
                start_time,
            ));
        }
        match graph.send_follow_up(&agent_path, &message).await {
            Ok(receipt) => {
                let output = format!(
                    "Queued follow-up for {} in turn {} and delivered it to the live mailbox.",
                    receipt.agent_path, receipt.turn_id
                );
                Ok(
                    messaging_timed_result(call, self.name(), output, start_time)
                        .with_metadata("operation", json!("follow_up"))
                        .with_metadata("status", json!("queued"))
                        .with_metadata("agent_path", json!(receipt.agent_path.as_path_str()))
                        .with_metadata("child_thread_id", json!(receipt.child_thread_id))
                        .with_metadata("turn_id", json!(receipt.turn_id))
                        .with_metadata("item_id", json!(receipt.item_id))
                        .with_metadata("sequence", json!(receipt.sequence))
                        .with_metadata("delivery", json!("live_mailbox")),
                )
            }
            Err(err) => Ok(graph_error_result(
                call,
                self.name(),
                "follow_up",
                agent_path.as_path_str(),
                err,
                start_time,
            )),
        }
    }

    async fn execute_interrupt(
        &self,
        call: &ToolCall,
        graph: &SubAgentGraph,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let agent_path =
            parse_messaging_agent_path(required_messaging_string(call, "agent_path")?)?;
        let reason = normalized_interrupt_reason(call.get_string("reason"));
        self.task_registry.reconcile_finished_tasks().await;
        if let Some(result) = self
            .registry_terminal_result(call, &agent_path, graph, start_time)
            .await?
        {
            return Ok(result);
        }

        match graph.interrupt_child(&agent_path, Some(&reason)).await {
            Ok(receipt) => {
                let interrupted_live_task = self
                    .task_registry
                    .interrupt_task(&receipt.child_thread_id, Some(reason));
                let output = format!("Interrupted agent {}.", receipt.agent_path);
                Ok(
                    messaging_timed_result(call, self.name(), output, start_time)
                        .with_metadata("operation", json!("interrupt"))
                        .with_metadata("status", json!("interrupted"))
                        .with_metadata("agent_path", json!(receipt.agent_path.as_path_str()))
                        .with_metadata("child_thread_id", json!(receipt.child_thread_id))
                        .with_metadata("turn_id", json!(receipt.turn_id))
                        .with_metadata("item_id", json!(receipt.item_id))
                        .with_metadata("sequence", json!(receipt.sequence))
                        .with_metadata("interrupted_live_task", json!(interrupted_live_task)),
                )
            }
            Err(err) => Ok(graph_error_result(
                call,
                self.name(),
                "interrupt",
                agent_path.as_path_str(),
                err,
                start_time,
            )),
        }
    }

    async fn registry_terminal_result(
        &self,
        call: &ToolCall,
        agent_path: &AgentPath,
        graph: &SubAgentGraph,
        start_time: Instant,
    ) -> Result<Option<ToolResult>, ToolError> {
        let summary = match graph.read_child(agent_path).await {
            Ok(summary) => summary,
            Err(err) => {
                return Ok(Some(graph_error_result(
                    call,
                    self.name(),
                    "interrupt",
                    agent_path.as_path_str(),
                    err,
                    start_time,
                )));
            }
        };
        let Some(task) = self.task_registry.get_task(&summary.child_thread_id) else {
            return Ok(None);
        };
        if is_task_terminal(&task) {
            let result = messaging_invalid_state_result(
                call,
                self.name(),
                "interrupt",
                agent_path.as_path_str(),
                task.status.to_string(),
                start_time,
            );
            return Ok(Some(result));
        }
        Ok(None)
    }
}

fn messaging_timed_result(
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

fn graph_error_result(
    call: &ToolCall,
    tool_name: &str,
    operation: &str,
    agent_path: &str,
    err: SubAgentGraphError,
    start_time: Instant,
) -> ToolResult {
    match err {
        SubAgentGraphError::InvalidAgentState { status, .. } => messaging_invalid_state_result(
            call, tool_name, operation, agent_path, status, start_time,
        ),
        SubAgentGraphError::ThreadStore(ThreadStoreError::ThreadNotFound(_)) => {
            messaging_error_result(
                messaging_timed_result(
                    call,
                    tool_name,
                    format!("Agent {agent_path} was not found."),
                    start_time,
                )
                .with_metadata("operation", json!(operation))
                .with_metadata("error_code", json!("agent_not_found"))
                .with_metadata("agent_path", json!(agent_path)),
            )
        }
        SubAgentGraphError::EmptyAgentMessage => messaging_error_result(
            messaging_timed_result(
                call,
                tool_name,
                "Agent message cannot be empty.".to_string(),
                start_time,
            )
            .with_metadata("operation", json!(operation))
            .with_metadata("error_code", json!("invalid_message"))
            .with_metadata("agent_path", json!(agent_path)),
        ),
        other => messaging_error_result(
            messaging_timed_result(
                call,
                tool_name,
                format!("Agent {operation} failed for {agent_path}: {other}"),
                start_time,
            )
            .with_metadata("operation", json!(operation))
            .with_metadata("error_code", json!("graph_error"))
            .with_metadata("agent_path", json!(agent_path)),
        ),
    }
}

fn messaging_invalid_state_result(
    call: &ToolCall,
    tool_name: &str,
    operation: &str,
    agent_path: &str,
    status: String,
    start_time: Instant,
) -> ToolResult {
    messaging_error_result(
        messaging_timed_result(
            call,
            tool_name,
            format!("Agent {agent_path} is in invalid state {status} for {operation}."),
            start_time,
        )
        .with_metadata("operation", json!(operation))
        .with_metadata("error_code", json!("invalid_state"))
        .with_metadata("agent_path", json!(agent_path))
        .with_metadata("status", json!(status)),
    )
}

fn messaging_unsupported_result(
    call: &ToolCall,
    tool_name: &str,
    agent_path: &str,
    start_time: Instant,
) -> ToolResult {
    messaging_error_result(
        messaging_timed_result(
            call,
            tool_name,
            format!(
                "Agent {agent_path} cannot accept follow-up without a live graph-backed task registry entry."
            ),
            start_time,
        )
        .with_metadata("operation", json!("follow_up"))
        .with_metadata("error_code", json!("unsupported_follow_up"))
        .with_metadata("agent_path", json!(agent_path)),
    )
}

fn messaging_error_result(mut result: ToolResult) -> ToolResult {
    (result.success, result.error, result.exit_code) = (false, result.output.clone(), Some(1));
    result.metadata.insert("retryable".into(), json!(false));
    result
}

fn messaging_operation(call: &ToolCall) -> Result<String, ToolError> {
    required_messaging_string(call, "operation")
}

fn required_messaging_string(call: &ToolCall, name: &str) -> Result<String, ToolError> {
    call.get_string(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ToolError::InvalidArguments(format!("missing required '{name}'")))
}

fn normalized_interrupt_reason(reason: Option<String>) -> String {
    reason
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_INTERRUPT_REASON.to_string())
}

fn validate_messaging_non_empty(name: &str, value: &str) -> Result<(), ToolError> {
    (!value.trim().is_empty())
        .then_some(())
        .ok_or_else(|| ToolError::InvalidArguments(format!("{name} cannot be empty")))
}

fn parse_messaging_agent_path(raw: String) -> Result<AgentPath, ToolError> {
    AgentPath::from_raw_path(raw).map_err(|err| ToolError::InvalidArguments(err.to_string()))
}

fn is_task_terminal(task: &TaskRequest) -> bool {
    matches!(
        task.status,
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Interrupted
    )
}

fn is_task_active(task: &TaskRequest) -> bool {
    matches!(task.status, TaskStatus::Pending | TaskStatus::Running)
}

fn is_graph_terminal(status: ThreadStatus) -> bool {
    matches!(
        status,
        ThreadStatus::Completed | ThreadStatus::Failed | ThreadStatus::Interrupted
    )
}

#[cfg(test)]
#[path = "agent_messaging_tests.rs"]
mod tests;
