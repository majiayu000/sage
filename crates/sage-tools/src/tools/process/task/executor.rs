//! Task execution logic for running subagents

use anyhow::{anyhow, bail};
use sage_core::agent::subagent::{
    AgentPath, AgentType, ChildAgentSpawnRecord, ChildAgentSummary, SubAgentConfig, SubAgentGraph,
    SubAgentGraphError, Thoroughness, execute_subagent,
};
use sage_core::thread_store::ThreadStoreError;
use sage_core::tools::permission::ToolContext;
use sage_core::tools::types::{ToolCall, ToolResult};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

use super::types::{TaskRegistry, TaskRequest, TaskStatus};

/// Execute a task synchronously
pub async fn execute_task_sync(
    call: &ToolCall,
    registry: Arc<TaskRegistry>,
) -> anyhow::Result<ToolResult> {
    let (task_params, agent_type, thoroughness) = parse_task_parameters(call)?;

    // Generate task ID
    let task_id = task_params
        .resume
        .clone()
        .unwrap_or_else(|| format!("task_{}", Uuid::new_v4()));

    // Create and register task
    let task = TaskRequest {
        id: task_id.clone(),
        description: task_params.description.clone(),
        prompt: task_params.prompt.clone(),
        subagent_type: task_params.subagent_type.clone(),
        model: task_params.model.clone(),
        run_in_background: false,
        resume: task_params.resume.clone(),
        status: TaskStatus::Running,
        result: None,
    };

    registry.add_task(task);

    // Execute subagent
    let config =
        SubAgentConfig::new(agent_type, task_params.prompt.clone()).with_thoroughness(thoroughness);

    let result = execute_subagent(config).await;

    match result {
        Ok(result) => {
            // Update task status
            registry.update_status(
                &task_id,
                TaskStatus::Completed,
                Some(result.content.clone()),
            );

            let response = format!(
                "## Sub-agent Result ({})\n\n\
                 **Agent ID**: {}\n\
                 **Execution Time**: {}ms\n\
                 **Tools Used**: {}\n\
                 **Total Tool Calls**: {}\n\n\
                 ---\n\n\
                 {}",
                task_params.subagent_type,
                result.agent_id,
                result.metadata.execution_time_ms,
                result.metadata.tools_used.join(", "),
                result.metadata.total_tool_uses,
                result.content
            );

            Ok(ToolResult::success(&call.id, "Task", response)
                .with_execution_time(result.metadata.execution_time_ms)
                .with_metadata("task_id", json!(task_id))
                .with_metadata("agent_id", json!(result.agent_id))
                .with_metadata("subagent_type", json!(task_params.subagent_type))
                .with_metadata("tools_used", json!(result.metadata.tools_used))
                .with_metadata("total_tool_uses", json!(result.metadata.total_tool_uses)))
        }
        Err(e) => {
            // Update task status to failed
            registry.update_status(&task_id, TaskStatus::Failed, Some(e.to_string()));

            // Check if runner is not initialized
            let error_msg = if e.to_string().contains("not initialized") {
                format!(
                    "Sub-agent execution failed: Runner not initialized.\n\n\
                     This usually means the agent was started without sub-agent support.\n\
                     Error: {}",
                    e
                )
            } else {
                format!("Sub-agent execution failed: {}", e)
            };

            Ok(ToolResult::error(&call.id, "Task", error_msg)
                .with_metadata("task_id", json!(task_id))
                .with_metadata("subagent_type", json!(task_params.subagent_type)))
        }
    }
}

/// Execute a task in background
pub async fn execute_task_background(
    call: &ToolCall,
    registry: Arc<TaskRegistry>,
) -> anyhow::Result<ToolResult> {
    let (task_params, agent_type, thoroughness) = parse_task_parameters(call)?;

    // Generate task ID
    let task_id = task_params
        .resume
        .clone()
        .unwrap_or_else(|| format!("task_{}", Uuid::new_v4()));

    // Create and register task
    let task = TaskRequest {
        id: task_id.clone(),
        description: task_params.description.clone(),
        prompt: task_params.prompt.clone(),
        subagent_type: task_params.subagent_type.clone(),
        model: task_params.model.clone(),
        run_in_background: true,
        resume: task_params.resume.clone(),
        status: TaskStatus::Running,
        result: None,
    };

    registry.add_task(task);

    let config =
        SubAgentConfig::new(agent_type, task_params.prompt.clone()).with_thoroughness(thoroughness);
    let background_task_id = task_id.clone();
    let background_registry = Arc::downgrade(&registry);
    let handle = tokio::spawn(async move {
        let result = execute_subagent(config.with_background(true)).await;
        match result {
            Ok(result) => {
                update_background_status(
                    &background_registry,
                    &background_task_id,
                    TaskStatus::Completed,
                    Some(result.content),
                );
            }
            Err(err) => {
                update_background_status(
                    &background_registry,
                    &background_task_id,
                    TaskStatus::Failed,
                    Some(err.to_string()),
                );
            }
        }
    });
    registry.register_handle(task_id.clone(), handle);

    let response = format!(
        "Task '{}' ({}) started in background.\n\
         Agent type: {}\n\
         Task ID: {}\n\n\
         Use TaskOutput with task_id=\"{}\" to retrieve results when ready.",
        task_params.description, task_id, task_params.subagent_type, task_id, task_id
    );

    Ok(ToolResult::success(&call.id, "Task", response)
        .with_metadata("task_id", json!(task_id))
        .with_metadata("subagent_type", json!(task_params.subagent_type))
        .with_metadata("run_in_background", json!(true)))
}

/// Execute a task in background and persist the sub-agent edge in the graph.
pub async fn execute_task_background_with_graph(
    call: &ToolCall,
    registry: Arc<TaskRegistry>,
    graph: Arc<SubAgentGraph>,
    context: &ToolContext,
) -> anyhow::Result<ToolResult> {
    let (task_params, agent_type, thoroughness) = parse_task_parameters(call)?;
    let parent_thread_id = context
        .session_id
        .as_deref()
        .ok_or_else(|| anyhow!("Task background graph spawn requires session_id context"))?;

    // Generate task ID
    let task_id = task_params
        .resume
        .clone()
        .unwrap_or_else(|| format!("task_{}", Uuid::new_v4()));

    let summary = record_or_reuse_graph_child(
        &graph,
        parent_thread_id,
        &task_id,
        &call.id,
        &task_params.description,
        task_params.resume.is_some(),
    )
    .await?;

    // Create and register task only after the graph edge is durable.
    let task = TaskRequest {
        id: task_id.clone(),
        description: task_params.description.clone(),
        prompt: task_params.prompt.clone(),
        subagent_type: task_params.subagent_type.clone(),
        model: task_params.model.clone(),
        run_in_background: true,
        resume: task_params.resume.clone(),
        status: TaskStatus::Running,
        result: None,
    };

    registry.add_task(task);

    let config =
        SubAgentConfig::new(agent_type, task_params.prompt.clone()).with_thoroughness(thoroughness);
    let background_task_id = task_id.clone();
    let background_registry = Arc::downgrade(&registry);
    let handle = tokio::spawn(async move {
        let result = execute_subagent(config.with_background(true)).await;
        match result {
            Ok(result) => {
                update_background_status(
                    &background_registry,
                    &background_task_id,
                    TaskStatus::Completed,
                    Some(result.content),
                );
            }
            Err(err) => {
                update_background_status(
                    &background_registry,
                    &background_task_id,
                    TaskStatus::Failed,
                    Some(err.to_string()),
                );
            }
        }
    });
    registry.register_handle(task_id.clone(), handle);

    let response = format!(
        "Task '{}' ({}) started in background.\n\
         Agent type: {}\n\
         Task ID: {}\n\
         Agent path: {}\n\n\
         Use TaskOutput with task_id=\"{}\" or agent_path=\"{}\" to retrieve results when ready.",
        task_params.description,
        task_id,
        task_params.subagent_type,
        task_id,
        summary.agent_path,
        task_id,
        summary.agent_path
    );

    Ok(ToolResult::success(&call.id, "Task", response)
        .with_metadata("task_id", json!(task_id))
        .with_metadata("agent_path", json!(summary.agent_path.as_path_str()))
        .with_metadata("parent_thread_id", json!(summary.parent_thread_id))
        .with_metadata("child_thread_id", json!(summary.child_thread_id))
        .with_metadata("spawn_item_id", json!(summary.spawn_item_id))
        .with_metadata("subagent_type", json!(task_params.subagent_type))
        .with_metadata("run_in_background", json!(true)))
}

fn update_background_status(
    registry: &Weak<TaskRegistry>,
    task_id: &str,
    status: TaskStatus,
    result: Option<String>,
) {
    if let Some(registry) = registry.upgrade() {
        registry.update_status(task_id, status, result);
    }
}

async fn record_or_reuse_graph_child(
    graph: &SubAgentGraph,
    parent_thread_id: &str,
    task_id: &str,
    spawn_item_id: &str,
    title: &str,
    resume_existing: bool,
) -> anyhow::Result<ChildAgentSummary> {
    if resume_existing {
        let agent_path = AgentPath::try_for_child_thread(task_id)?;
        match graph.read_child(&agent_path).await {
            Ok(summary) => {
                if summary.parent_thread_id != parent_thread_id {
                    bail!(
                        "Task resume '{}' is linked to parent thread '{}' not '{}'",
                        task_id,
                        summary.parent_thread_id,
                        parent_thread_id
                    );
                }
                return Ok(summary);
            }
            Err(SubAgentGraphError::ThreadStore(ThreadStoreError::ThreadNotFound(_))) => {}
            Err(err) => return Err(err.into()),
        }
    }

    let mut spawn = ChildAgentSpawnRecord::new(parent_thread_id, task_id, spawn_item_id);
    spawn.title = Some(title.to_string());
    Ok(graph.record_child(spawn).await?)
}

/// Task parameters parsed from tool call
struct TaskParameters {
    description: String,
    prompt: String,
    subagent_type: String,
    model: Option<String>,
    resume: Option<String>,
}

/// Parse task parameters from tool call
fn parse_task_parameters(
    call: &ToolCall,
) -> anyhow::Result<(TaskParameters, AgentType, Thoroughness)> {
    let description = call
        .arguments
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'description' parameter"))?
        .to_string();

    let prompt = call
        .arguments
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'prompt' parameter"))?
        .to_string();

    let subagent_type = call
        .arguments
        .get("subagent_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'subagent_type' parameter"))?
        .to_string();

    let model = call
        .arguments
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let resume = call
        .arguments
        .get("resume")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse thoroughness level
    let thoroughness = call
        .arguments
        .get("thoroughness")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "quick" => Thoroughness::Quick,
            "very_thorough" | "very-thorough" | "thorough" => Thoroughness::VeryThorough,
            _ => Thoroughness::Medium,
        })
        .unwrap_or(Thoroughness::Medium);

    // Parse agent type
    let agent_type = match subagent_type.to_lowercase().as_str() {
        "explore" => AgentType::Explore,
        "plan" => AgentType::Plan,
        "general-purpose" | "general_purpose" | "general" => AgentType::GeneralPurpose,
        _ => AgentType::GeneralPurpose,
    };

    let params = TaskParameters {
        description,
        prompt,
        subagent_type,
        model,
        resume,
    };

    Ok((params, agent_type, thoroughness))
}
