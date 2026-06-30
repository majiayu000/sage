use sage_core::agent::subagent::{AgentTerminalState, ChildAgentSummary};
use serde_json::json;
use std::time::{Duration, Instant};

use super::*;

impl TaskOutputTool {
    pub(super) async fn execute_subagent_task_id_output(
        &self,
        call: &ToolCall,
        task_id: &str,
        block: bool,
        timeout: Duration,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        match self
            .execute_subagent_task_output(call, task_id, None, block, timeout, start_time)
            .await
        {
            Ok(result) => Ok(result),
            Err(ToolError::NotFound(message)) => {
                let Some(graph) = self.subagent_graph.as_ref() else {
                    return Err(ToolError::NotFound(message));
                };
                let agent_path =
                    sage_core::agent::subagent::AgentPath::try_for_child_thread(task_id)
                        .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
                if graph.read_child(&agent_path).await.is_err() {
                    return Err(ToolError::NotFound(message));
                }
                let (summary, terminal) = self
                    .read_durable_summary_and_terminal(graph, agent_path, block, timeout)
                    .await?;
                Ok(self.execute_durable_subagent_output(call, summary, terminal, start_time))
            }
            Err(err) => Err(err),
        }
    }

    pub(super) async fn execute_subagent_agent_path_output(
        &self,
        call: &ToolCall,
        agent_path_raw: &str,
        block: bool,
        timeout: Duration,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let graph = self.subagent_graph.as_ref().ok_or_else(|| {
            ToolError::InvalidArguments(
                "agent_path requires TaskOutputTool configured with a SubAgentGraph".to_string(),
            )
        })?;
        let agent_path = parse_agent_path(agent_path_raw)?;
        let summary = graph.read_child(&agent_path).await.map_err(|err| {
            ToolError::NotFound(format!(
                "Sub-agent '{}' not found in graph: {}",
                agent_path.as_path_str(),
                err
            ))
        })?;
        let task_id = summary.child_thread_id.clone();

        match self
            .execute_subagent_task_output(
                call,
                &task_id,
                Some(summary.clone()),
                block,
                timeout,
                start_time,
            )
            .await
        {
            Ok(result) => Ok(result),
            Err(ToolError::NotFound(_)) => {
                let (summary, terminal) = self
                    .read_durable_summary_and_terminal(graph, agent_path, block, timeout)
                    .await?;
                Ok(self.execute_durable_subagent_output(call, summary, terminal, start_time))
            }
            Err(err) => Err(err),
        }
    }

    async fn read_durable_summary_and_terminal(
        &self,
        graph: &SubAgentGraph,
        agent_path: AgentPath,
        block: bool,
        timeout: Duration,
    ) -> Result<(ChildAgentSummary, Option<AgentTerminalState>), ToolError> {
        let started = Instant::now();
        loop {
            let summary = graph.read_child(&agent_path).await.map_err(|err| {
                ToolError::ExecutionFailed(format!(
                    "failed to read durable summary for '{}': {}",
                    agent_path.as_path_str(),
                    err
                ))
            })?;
            let terminal = graph
                .read_terminal_state(&agent_path)
                .await
                .map_err(|err| {
                    ToolError::ExecutionFailed(format!(
                        "failed to read durable output for '{}': {}",
                        agent_path.as_path_str(),
                        err
                    ))
                })?;
            if terminal.is_some()
                || is_graph_terminal(summary.status)
                || !block
                || started.elapsed() >= timeout
            {
                return Ok((summary, terminal));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn execute_durable_subagent_output(
        &self,
        call: &ToolCall,
        summary: ChildAgentSummary,
        terminal: Option<AgentTerminalState>,
        start_time: Instant,
    ) -> ToolResult {
        let status = terminal
            .as_ref()
            .map(|terminal| terminal.status.as_str().to_string())
            .unwrap_or_else(|| summary.status.as_str().to_string());
        let final_result = terminal.as_ref().and_then(|terminal| {
            (terminal.status == sage_core::thread_store::ThreadStatus::Completed)
                .then(|| terminal.result.clone())
                .flatten()
        });
        let error = terminal.as_ref().and_then(|terminal| {
            matches!(
                terminal.status,
                sage_core::thread_store::ThreadStatus::Failed
                    | sage_core::thread_store::ThreadStatus::Interrupted
            )
            .then(|| terminal.result.clone().or_else(|| terminal.reason.clone()))
            .flatten()
        });
        let event_summary = format!(
            "Sub-agent '{}' is {} from durable graph state.",
            summary.child_thread_id, status
        );
        let output = format!(
            "Task ID: {}\nStatus: {}\nAgent path: {}\n\n--- EVENT SUMMARY ---\n{}\n\n--- STDOUT PREVIEW ---\n(empty)\n--- STDERR PREVIEW ---\n(empty)\n--- ERROR ---\n{}\n--- FINAL RESULT ---\n{}",
            summary.child_thread_id,
            status,
            summary.agent_path,
            event_summary,
            error.as_deref().unwrap_or("(empty)"),
            final_result.as_deref().unwrap_or("(empty)")
        );
        let mut result = ToolResult::success(&call.id, self.name(), output)
            .with_metadata("task_id", json!(summary.child_thread_id))
            .with_metadata("status", json!(status))
            .with_metadata("event_summary", json!(event_summary))
            .with_metadata("stdout_preview", json!(""))
            .with_metadata("stderr_preview", json!(""))
            .with_metadata("error", json!(error))
            .with_metadata("final_result", json!(final_result))
            .with_metadata("agent_path", json!(summary.agent_path.as_path_str()))
            .with_metadata("parent_thread_id", json!(summary.parent_thread_id))
            .with_metadata("child_thread_id", json!(summary.child_thread_id))
            .with_metadata("spawn_item_id", json!(summary.spawn_item_id))
            .with_metadata("graph_status", json!(summary.status.as_str()));
        if let Some(terminal) = terminal {
            result = result
                .with_metadata("terminal_item_id", json!(terminal.item_id))
                .with_metadata("terminal_sequence", json!(terminal.sequence));
        }
        result.execution_time_ms =
            Some(u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX));
        result
    }
}

fn is_graph_terminal(status: sage_core::thread_store::ThreadStatus) -> bool {
    matches!(
        status,
        sage_core::thread_store::ThreadStatus::Completed
            | sage_core::thread_store::ThreadStatus::Failed
            | sage_core::thread_store::ThreadStatus::Interrupted
    )
}
