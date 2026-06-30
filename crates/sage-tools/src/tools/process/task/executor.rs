//! Task execution logic for running subagents

use anyhow::{anyhow, bail};
use sage_core::agent::subagent::{
    AgentPath, ChildAgentSpawnRecord, ChildAgentSummary, SubAgentGraph, SubAgentGraphError,
    execute_subagent, execute_subagent_with_mailbox,
};
use sage_core::thread_store::{ThreadStatus, ThreadStoreError};
use sage_core::tools::permission::ToolContext;
use sage_core::tools::types::{ToolCall, ToolResult};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

use super::spawn_context::{
    prepare_subagent_config, resolve_parent_context, resolve_parent_context_allow_missing_thread,
};
use super::spawn_params::parse_task_parameters;
use super::types::{TaskRegistry, TaskRequest, TaskStatus};

/// Execute a task synchronously
pub async fn execute_task_sync(
    call: &ToolCall,
    registry: Arc<TaskRegistry>,
    graph: Option<Arc<SubAgentGraph>>,
    context: Option<&ToolContext>,
) -> anyhow::Result<ToolResult> {
    let (task_params, agent_type, thoroughness) = parse_task_parameters(call)?;
    let parent_context =
        resolve_parent_context_allow_missing_thread(graph.as_deref(), context).await?;
    let config = prepare_subagent_config(
        &task_params,
        agent_type,
        thoroughness,
        parent_context.messages.clone(),
        context,
    )?;

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

    let result = execute_subagent(config).await;

    match result {
        Ok(result) => {
            let role_resolution = result.metadata.role_resolution.as_deref();
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
                .with_metadata(
                    "role_name",
                    json!(role_resolution.and_then(|role| role.role_name.as_deref())),
                )
                .with_metadata(
                    "role_source",
                    json!(role_resolution.and_then(|role| role.role_source.as_deref())),
                )
                .with_metadata(
                    "model",
                    json!(role_resolution.and_then(|role| role.model.as_deref())),
                )
                .with_metadata(
                    "reasoning",
                    json!(role_resolution.and_then(|role| role.reasoning.as_deref())),
                )
                .with_metadata(
                    "profile",
                    json!(role_resolution.and_then(|role| role.profile.as_deref())),
                )
                .with_metadata(
                    "fork_context",
                    json!(role_resolution.and_then(|role| role.fork_context.as_deref())),
                )
                .with_metadata(
                    "forked_messages",
                    json!(role_resolution.map(|role| role.forked_messages)),
                )
                .with_metadata(
                    "available_tools",
                    json!(role_resolution.map(|role| &role.available_tools)),
                )
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
    context: Option<&ToolContext>,
) -> anyhow::Result<ToolResult> {
    let (task_params, agent_type, thoroughness) = parse_task_parameters(call)?;
    let parent_context = resolve_parent_context(None, context).await?;
    let config = prepare_subagent_config(
        &task_params,
        agent_type,
        thoroughness,
        parent_context.messages.clone(),
        context,
    )?;

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
        .with_metadata("role_path", json!(task_params.role_path))
        .with_metadata("fork_context", json!(task_params.fork_context))
        .with_metadata("parent_context_source", json!(parent_context.source))
        .with_metadata("parent_context_messages", json!(parent_context.count()))
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
    let parent_context = resolve_parent_context(Some(graph.as_ref()), Some(context)).await?;
    let config = prepare_subagent_config(
        &task_params,
        agent_type,
        thoroughness,
        parent_context.messages.clone(),
        Some(context),
    )?;

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

    let background_task_id = task_id.clone();
    let background_agent_path = summary.agent_path.clone();
    let background_graph = graph.clone();
    let background_registry = Arc::downgrade(&registry);
    let handle = tokio::spawn(async move {
        let result = execute_subagent_with_mailbox(
            config.with_background(true),
            background_graph.clone(),
            background_agent_path.clone(),
        )
        .await;
        match result {
            Ok(result) => {
                let content = result.content;
                let graph_result = background_graph
                    .record_terminal_state(
                        &background_agent_path,
                        ThreadStatus::Completed,
                        Some(&content),
                    )
                    .await;
                let (status, result) =
                    graph_task_result(graph_result, TaskStatus::Completed, content);
                update_background_status(
                    &background_registry,
                    &background_task_id,
                    status,
                    Some(result),
                );
            }
            Err(err) => {
                let message = err.to_string();
                let graph_result = background_graph
                    .record_terminal_state(
                        &background_agent_path,
                        ThreadStatus::Failed,
                        Some(&message),
                    )
                    .await;
                let (status, result) = graph_task_result(graph_result, TaskStatus::Failed, message);
                update_background_status(
                    &background_registry,
                    &background_task_id,
                    status,
                    Some(result),
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
        .with_metadata("role_path", json!(task_params.role_path))
        .with_metadata("fork_context", json!(task_params.fork_context))
        .with_metadata("parent_context_source", json!(parent_context.source))
        .with_metadata("parent_context_messages", json!(parent_context.count()))
        .with_metadata("run_in_background", json!(true)))
}

fn update_background_status(
    registry: &Weak<TaskRegistry>,
    task_id: &str,
    status: TaskStatus,
    result: Option<String>,
) {
    if let Some(registry) = registry.upgrade() {
        if registry
            .get_task(task_id)
            .is_some_and(|task| task.status == TaskStatus::Interrupted)
        {
            return;
        }
        registry.update_status(task_id, status, result);
    }
}

fn graph_task_result(
    result: Result<impl Sized, sage_core::agent::subagent::SubAgentGraphError>,
    fallback: TaskStatus,
    fallback_result: String,
) -> (TaskStatus, String) {
    match result {
        Ok(_) => (fallback, fallback_result),
        Err(err) => (
            TaskStatus::Failed,
            format!("failed to persist terminal graph status: {err}; result: {fallback_result}"),
        ),
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
