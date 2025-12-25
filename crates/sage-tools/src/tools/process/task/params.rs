//! Parameter parsing for task execution

use sage_core::agent::subagent::{AgentType, Thoroughness};
use sage_core::tools::types::ToolCall;

/// Task parameters parsed from tool call
pub struct TaskParameters {
    pub description: String,
    pub prompt: String,
    pub subagent_type: String,
    pub model: Option<String>,
    pub resume: Option<String>,
}

/// Parse task parameters from tool call
pub fn parse_task_parameters(
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
