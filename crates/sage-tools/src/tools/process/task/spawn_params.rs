//! Task spawn parameter parsing and sub-agent config construction.

use anyhow::{anyhow, bail};
use sage_core::agent::subagent::types::{
    validate_model_override, validate_profile_override, validate_reasoning_override,
};
use sage_core::agent::subagent::{
    AgentType, ForkContextMessage, ForkContextPolicy, SubAgentConfig, Thoroughness,
};
use sage_core::tools::permission::ToolContext;
use sage_core::tools::types::ToolCall;
use sage_core::types::MessageRole;
use serde_json::Value;
use std::path::PathBuf;

/// Task parameters parsed from tool call.
#[derive(Debug, Clone)]
pub(super) struct TaskParameters {
    pub description: String,
    pub prompt: String,
    pub subagent_type: String,
    pub model: Option<String>,
    pub reasoning: Option<String>,
    pub profile: Option<String>,
    pub role_path: Option<PathBuf>,
    pub fork_context: Option<ForkContextPolicy>,
    pub resume: Option<String>,
}

pub(super) fn parse_task_parameters(
    call: &ToolCall,
) -> anyhow::Result<(TaskParameters, AgentType, Thoroughness)> {
    let description = required_string(call, "description")?;
    let prompt = required_string(call, "prompt")?;
    let subagent_type = required_string(call, "subagent_type")?;
    let model = optional_string(call, "model");
    let reasoning = optional_string(call, "reasoning");
    let profile = optional_string(call, "profile");
    validate_optional_override("model", model.as_deref(), validate_model_override)?;
    validate_optional_override(
        "reasoning",
        reasoning.as_deref(),
        validate_reasoning_override,
    )?;
    validate_optional_override("profile", profile.as_deref(), validate_profile_override)?;
    let role_path = optional_string(call, "role_path").map(PathBuf::from);
    let resume = optional_string(call, "resume");
    let fork_context = call
        .arguments
        .get("fork_context")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|err| anyhow!("invalid fork_context: {err}"))?;
    let thoroughness = parse_thoroughness(call)?;
    let agent_type = parse_agent_type(&subagent_type, role_path.is_some())?;

    Ok((
        TaskParameters {
            description,
            prompt,
            subagent_type,
            model,
            reasoning,
            profile,
            role_path,
            fork_context,
            resume,
        },
        agent_type,
        thoroughness,
    ))
}

pub(super) fn build_subagent_config(
    params: &TaskParameters,
    agent_type: AgentType,
    thoroughness: Thoroughness,
    context_parent_context: Option<Vec<ForkContextMessage>>,
) -> SubAgentConfig {
    let mut config =
        SubAgentConfig::new(agent_type, params.prompt.clone()).with_thoroughness(thoroughness);
    if let Some(model) = &params.model {
        config = config.with_model(model.clone());
    }
    if let Some(reasoning) = &params.reasoning {
        config = config.with_reasoning(reasoning.clone());
    }
    if let Some(profile) = &params.profile {
        config = config.with_profile(profile.clone());
    }
    if let Some(path) = &params.role_path {
        config = config.with_role_path(path.clone());
    }
    if let Some(policy) = &params.fork_context {
        config = config.with_fork_context(policy.clone());
    }
    if let Some(parent_context) = context_parent_context {
        config = config.with_forked_parent_context(parent_context);
    }
    config
}

pub(super) fn parent_context_from_tool_context(
    context: &ToolContext,
) -> anyhow::Result<Option<Vec<ForkContextMessage>>> {
    context
        .metadata
        .get("parent_context")
        .map(parse_parent_context)
        .transpose()
}

fn required_string(call: &ToolCall, key: &str) -> anyhow::Result<String> {
    call.arguments
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Missing '{key}' parameter"))
}

fn optional_string(call: &ToolCall, key: &str) -> Option<String> {
    call.arguments
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn validate_optional_override(
    label: &str,
    value: Option<&str>,
    validate: fn(&str) -> sage_core::error::SageResult<()>,
) -> anyhow::Result<()> {
    if let Some(value) = value {
        validate(value).map_err(|err| anyhow!("invalid {label}: {err}"))?;
    }
    Ok(())
}

fn parse_thoroughness(call: &ToolCall) -> anyhow::Result<Thoroughness> {
    Ok(match optional_string(call, "thoroughness").as_deref() {
        Some("quick") => Thoroughness::Quick,
        Some("medium") | None => Thoroughness::Medium,
        Some("very_thorough" | "very-thorough" | "thorough") => Thoroughness::VeryThorough,
        Some(other) => bail!("unsupported thoroughness '{other}'"),
    })
}

fn parse_agent_type(subagent_type: &str, has_role_path: bool) -> anyhow::Result<AgentType> {
    if has_role_path {
        return Ok(AgentType::Custom);
    }
    match subagent_type.to_lowercase().as_str() {
        "explore" => Ok(AgentType::Explore),
        "plan" => Ok(AgentType::Plan),
        "general-purpose" | "general_purpose" | "general" => Ok(AgentType::GeneralPurpose),
        "custom" => bail!("custom subagent_type requires role_path"),
        other => bail!("unsupported subagent_type '{other}'"),
    }
}

fn parse_parent_context(value: &Value) -> anyhow::Result<Vec<ForkContextMessage>> {
    let items = value
        .as_array()
        .ok_or_else(|| anyhow!("parent_context must be an array"))?;
    items
        .iter()
        .map(|item| match item {
            Value::String(content) => ForkContextMessage::new(MessageRole::User, content.clone())
                .map_err(anyhow::Error::from),
            Value::Object(_) => serde_json::from_value(item.clone())
                .map_err(|err| anyhow!("invalid parent_context item: {err}")),
            _ => bail!("parent_context items must be strings or objects"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn call(arguments: serde_json::Value) -> ToolCall {
        ToolCall {
            id: "call-1".to_string(),
            name: "Task".to_string(),
            arguments: arguments
                .as_object()
                .expect("object")
                .clone()
                .into_iter()
                .collect::<HashMap<_, _>>(),
            call_id: None,
        }
    }

    #[test]
    fn task_spawn_params_parse_custom_role_and_fork_context() {
        let call = call(json!({
            "description": "Review code",
            "prompt": "Review this",
            "subagent_type": "custom",
            "role_path": "reviewer.toml",
            "fork_context": {"mode": "last_n", "turns": 2},
            "model": "haiku",
            "reasoning": "medium",
            "profile": "review"
        }));
        let (params, agent_type, thoroughness) =
            parse_task_parameters(&call).expect("parse params");
        assert_eq!(agent_type, AgentType::Custom);
        assert_eq!(thoroughness, Thoroughness::Medium);
        assert_eq!(
            params.role_path.as_deref(),
            Some(std::path::Path::new("reviewer.toml"))
        );
        assert!(matches!(
            params.fork_context,
            Some(ForkContextPolicy::LastN { turns: 2 })
        ));

        let config = build_subagent_config(&params, agent_type, thoroughness, None);
        assert_eq!(config.agent_type, AgentType::Custom);
        assert_eq!(config.model_override.as_deref(), Some("haiku"));
        assert_eq!(config.reasoning_override.as_deref(), Some("medium"));
        assert_eq!(config.profile_override.as_deref(), Some("review"));
        assert!(config.parent_context.is_empty());
    }

    #[test]
    fn task_spawn_params_unknown_subagent_fails_closed() {
        let call = call(json!({
            "description": "Do thing",
            "prompt": "Do it",
            "subagent_type": "unknown"
        }));
        let error = parse_task_parameters(&call).expect_err("unknown role must fail");
        assert!(error.to_string().contains("unsupported subagent_type"));
    }

    #[test]
    fn task_spawn_params_custom_requires_role_path() {
        let call = call(json!({
            "description": "Do thing",
            "prompt": "Do it",
            "subagent_type": "custom"
        }));
        let error = parse_task_parameters(&call).expect_err("custom role must need path");
        assert!(error.to_string().contains("requires role_path"));
    }

    #[test]
    fn task_spawn_params_uses_tool_context_parent_context() {
        let call = call(json!({
            "description": "Explore",
            "prompt": "Find it",
            "subagent_type": "Explore",
            "fork_context": "all"
        }));
        let (params, agent_type, thoroughness) =
            parse_task_parameters(&call).expect("parse params");
        let mut context = ToolContext::new(std::env::current_dir().expect("cwd"));
        context.metadata.insert(
            "parent_context".to_string(),
            json!([
                {"role": "user", "content": "parent task"},
                {"role": "assistant", "content": "parent answer"}
            ]),
        );
        let parent_context = parent_context_from_tool_context(&context).expect("context");
        let config = build_subagent_config(&params, agent_type, thoroughness, parent_context);
        assert_eq!(config.parent_context.len(), 2);
        assert!(matches!(config.fork_context, ForkContextPolicy::All));
    }
}
