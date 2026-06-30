//! TaskOutput tool for retrieving output from background shell and sub-agent tasks
//!
//! This tool allows retrieving stdout/stderr from background shell tasks and
//! structured status/results from sub-agent tasks started by the Task tool.

use async_trait::async_trait;
use sage_core::agent::subagent::{AgentPath, ChildAgentSummary, SubAgentGraph};
use sage_core::tools::BACKGROUND_REGISTRY;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::task::{GLOBAL_TASK_REGISTRY, TaskRegistry, TaskRequest, TaskStatus};

/// Tool for retrieving output from background shell tasks
pub struct TaskOutputTool {
    task_registry: Arc<TaskRegistry>,
    subagent_graph: Option<Arc<SubAgentGraph>>,
}

impl TaskOutputTool {
    /// Create a new TaskOutput tool
    pub fn new() -> Self {
        Self {
            task_registry: GLOBAL_TASK_REGISTRY.clone(),
            subagent_graph: None,
        }
    }

    /// Create a TaskOutput tool with an explicit task registry, useful for tests.
    pub fn with_task_registry(task_registry: Arc<TaskRegistry>) -> Self {
        Self {
            task_registry,
            subagent_graph: None,
        }
    }

    /// Create a TaskOutput tool with explicit task registry and sub-agent graph.
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
        r#"Retrieve output from a background shell task or sub-agent task.

Returns accumulated stdout and stderr from a background process started with
run_in_background=true in Bash. Also returns structured status, event summary,
error, and final result for Task sub-agents started with run_in_background=true.

Parameters:
- shell_id: The ID of the background shell task (e.g., "shell_1")
- task_id: The ID of the background sub-agent task (e.g., "task_...")
- agent_path: The stable sub-agent path (e.g., "agent://task_...")
- incremental: If true, only return output since last read (default: false)
- block: If true, wait for task completion (default: false)
- timeout: Max wait time in ms when blocking (default: 30000, max: 600000)

Example: task_output(task_id="task_...", block=true)"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "shell_id",
                    "The ID of the background shell task to get output from",
                )
                .optional(),
                ToolParameter::string(
                    "task_id",
                    "The ID of the background sub-agent task to get output from",
                )
                .optional(),
                ToolParameter::string("agent_path", "The stable sub-agent path to get output from")
                    .optional(),
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
        self.validate(call)?;
        let start_time = Instant::now();

        let incremental = call.get_bool("incremental").unwrap_or(false);
        let block = call.get_bool("block").unwrap_or(false);
        let timeout_raw = call.get_number("timeout").unwrap_or(30000.0);
        let timeout = timeout_duration(timeout_raw);

        if let Some(agent_path) = subagent_agent_path(call)? {
            return self
                .execute_subagent_agent_path_output(call, &agent_path, block, timeout, start_time)
                .await;
        }

        if let Some(task_id) = subagent_task_id(call) {
            return self
                .execute_subagent_task_output(call, &task_id, None, block, timeout, start_time)
                .await;
        }

        let shell_id = call.get_string("shell_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'shell_id' or 'task_id' parameter".to_string())
        })?;

        // Get task from registry
        let task = BACKGROUND_REGISTRY.get(&shell_id).ok_or_else(|| {
            ToolError::NotFound(format!("Background shell '{}' not found", shell_id))
        })?;

        // If blocking, wait for completion
        if block {
            let start = Instant::now();

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

    fn include_in_subagent_runner(&self) -> bool {
        self.subagent_graph.is_none()
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let shell_id = call.get_string("shell_id");
        let task_id = call.get_string("task_id");
        let agent_path = call.get_string("agent_path");

        let provided_count = [shell_id.is_some(), task_id.is_some(), agent_path.is_some()]
            .into_iter()
            .filter(|provided| *provided)
            .count();
        match provided_count {
            0 => {
                return Err(ToolError::InvalidArguments(
                    "Missing 'shell_id', 'task_id', or 'agent_path' parameter".to_string(),
                ));
            }
            1 => {}
            _ => {
                return Err(ToolError::InvalidArguments(
                    "Provide only one of 'shell_id', 'task_id', or 'agent_path'".to_string(),
                ));
            }
        }

        if let Some(shell_id) = shell_id {
            validate_identifier("shell_id", &shell_id)?;
        }
        if let Some(task_id) = task_id {
            validate_identifier("task_id", &task_id)?;
        }
        if let Some(agent_path) = agent_path {
            validate_agent_path(&agent_path)?;
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

impl TaskOutputTool {
    async fn execute_subagent_task_output(
        &self,
        call: &ToolCall,
        task_id: &str,
        graph_summary: Option<ChildAgentSummary>,
        block: bool,
        timeout: Duration,
        start_time: Instant,
    ) -> Result<ToolResult, ToolError> {
        let task = self.read_subagent_task(task_id, block, timeout).await?;
        let event_summary = format!("Sub-agent task '{}' is {}.", task.description, task.status);
        let error = if task.status == TaskStatus::Failed {
            task.result.clone()
        } else {
            None
        };
        let final_result = if task.status == TaskStatus::Completed {
            task.result.clone()
        } else {
            None
        };

        let output = format!(
            "Task ID: {}\nStatus: {}\nAgent type: {}\nBackground: {}\n\n--- EVENT SUMMARY ---\n{}\n\n--- STDOUT PREVIEW ---\n(empty)\n--- STDERR PREVIEW ---\n(empty)\n--- ERROR ---\n{}\n--- FINAL RESULT ---\n{}",
            task.id,
            task.status,
            task.subagent_type,
            task.run_in_background,
            event_summary,
            error.as_deref().unwrap_or("(empty)"),
            final_result.as_deref().unwrap_or("(empty)")
        );

        let mut result = ToolResult::success(&call.id, self.name(), output)
            .with_metadata("task_id", json!(task.id))
            .with_metadata("status", json!(task.status))
            .with_metadata("subagent_type", json!(task.subagent_type))
            .with_metadata("event_summary", json!(event_summary))
            .with_metadata("stdout_preview", json!(""))
            .with_metadata("stderr_preview", json!(""))
            .with_metadata("error", json!(error))
            .with_metadata("final_result", json!(final_result));
        if let Some(summary) = graph_summary {
            result = result
                .with_metadata("agent_path", json!(summary.agent_path.as_path_str()))
                .with_metadata("parent_thread_id", json!(summary.parent_thread_id))
                .with_metadata("child_thread_id", json!(summary.child_thread_id))
                .with_metadata("spawn_item_id", json!(summary.spawn_item_id))
                .with_metadata("graph_status", json!(summary.status));
        }
        result.execution_time_ms =
            Some(u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX));
        Ok(result)
    }

    async fn execute_subagent_agent_path_output(
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

        self.execute_subagent_task_output(call, &task_id, Some(summary), block, timeout, start_time)
            .await
    }

    async fn read_subagent_task(
        &self,
        task_id: &str,
        block: bool,
        timeout: Duration,
    ) -> Result<TaskRequest, ToolError> {
        let start = Instant::now();
        loop {
            self.task_registry.reconcile_finished_tasks().await;
            let task = self.task_registry.get_task(task_id).ok_or_else(|| {
                ToolError::NotFound(format!("Background task '{}' not found", task_id))
            })?;
            let is_running = matches!(task.status, TaskStatus::Pending | TaskStatus::Running);
            if !block || !is_running || start.elapsed() >= timeout {
                return Ok(task);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

fn timeout_duration(timeout_raw: f64) -> Duration {
    if timeout_raw.is_finite() && timeout_raw >= 0.0 {
        Duration::from_secs_f64(timeout_raw.min(600000.0) / 1000.0)
    } else {
        Duration::from_secs(30)
    }
}

fn validate_identifier(name: &str, value: &str) -> Result<(), ToolError> {
    if value.trim().is_empty() {
        return Err(ToolError::InvalidArguments(format!(
            "{name} cannot be empty"
        )));
    }

    if !value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ToolError::InvalidArguments(format!(
            "{name} must contain only alphanumeric characters, underscores, and hyphens"
        )));
    }

    Ok(())
}

fn parse_agent_path(value: &str) -> Result<AgentPath, ToolError> {
    AgentPath::from_raw_path(value).map_err(|err| ToolError::InvalidArguments(err.to_string()))
}

fn validate_agent_path(value: &str) -> Result<(), ToolError> {
    parse_agent_path(value).map(|_| ())
}

fn subagent_agent_path(call: &ToolCall) -> Result<Option<String>, ToolError> {
    call.get_string("agent_path")
        .map(|path| validate_agent_path(&path).map(|_| path))
        .transpose()
}

fn subagent_task_id(call: &ToolCall) -> Option<String> {
    call.get_string("task_id")
}

#[cfg(test)]
#[path = "task_output_tests.rs"]
mod tests;
