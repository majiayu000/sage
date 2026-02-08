//! Task execution logic for running subagents

use sage_core::agent::subagent::{AgentType, SubAgentConfig, Thoroughness, execute_subagent};
use sage_core::tools::types::{ToolCall, ToolResult};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use super::types::{TaskRegistry, TaskRequest, TaskStatus};

/// Execute a task synchronously
pub async fn execute_task_sync(
    call: &ToolCall,
    registry: Arc<TaskRegistry>,
) -> Result<ToolResult, String> {
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
pub fn execute_task_background(
    call: &ToolCall,
    registry: Arc<TaskRegistry>,
) -> Result<ToolResult, String> {
    let (task_params, _agent_type, _thoroughness) = parse_task_parameters(call)?;

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
        status: TaskStatus::Pending,
        result: None,
    };

    registry.add_task(task);

    let response = format!(
        "Task '{}' ({}) queued for background execution.\n\
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
) -> Result<(TaskParameters, AgentType, Thoroughness), String> {
    let description = call
        .arguments
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'description' parameter".to_string())?
        .to_string();

    let prompt = call
        .arguments
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'prompt' parameter".to_string())?
        .to_string();

    let subagent_type = call
        .arguments
        .get("subagent_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'subagent_type' parameter".to_string())?
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
