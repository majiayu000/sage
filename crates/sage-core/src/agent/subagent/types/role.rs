//! Declarative sub-agent role schema.

use super::{ForkContextPolicy, ToolAccessControl, WorkingDirectoryConfig};
use crate::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Local declarative role configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SubAgentRoleConfig {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub working_directory_policy: Option<WorkingDirectoryConfig>,
    #[serde(default)]
    pub fork_context: Option<ForkContextPolicy>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl SubAgentRoleConfig {
    pub fn validate(&self) -> SageResult<()> {
        validate_non_empty("role name", &self.name)?;
        validate_non_empty("role prompt", &self.prompt)?;
        validate_tool_names(&self.tools)?;

        if let Some(model) = &self.model {
            validate_model_override(model)?;
        }
        if let Some(reasoning) = &self.reasoning {
            validate_reasoning_override(reasoning)?;
        }
        if let Some(profile) = &self.profile {
            validate_profile_override(profile)?;
        }
        if let Some(fork_context) = &self.fork_context {
            fork_context.validate()?;
        }

        Ok(())
    }

    pub fn tool_access(&self) -> ToolAccessControl {
        if self.tools.is_empty() {
            ToolAccessControl::None
        } else {
            ToolAccessControl::Specific(self.tools.clone())
        }
    }
}

pub fn validate_model_override(model: &str) -> SageResult<()> {
    validate_non_empty("model override", model)?;
    validate_model_token(model)
}

pub fn validate_reasoning_override(reasoning: &str) -> SageResult<()> {
    match reasoning {
        "low" | "medium" | "high" | "xhigh" => Ok(()),
        other => Err(SageError::config(format!(
            "unsupported reasoning override '{other}'"
        ))),
    }
}

pub fn validate_profile_override(profile: &str) -> SageResult<()> {
    profile_tool_access(profile).map(|_| ())
}

pub fn profile_tool_access(profile: &str) -> SageResult<ToolAccessControl> {
    validate_token("profile override", profile)?;
    let tools = match profile {
        "default" | "inherit" => return Ok(ToolAccessControl::Inherited),
        "review" | "readonly" | "read_only" | "explore" => {
            vec!["Read".to_string(), "Grep".to_string(), "Glob".to_string()]
        }
        "plan" | "planning" => vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "Bash".to_string(),
            "TodoRead".to_string(),
            "TodoWrite".to_string(),
            "Task".to_string(),
            "TaskOutput".to_string(),
        ],
        "write" | "edit" => return Ok(ToolAccessControl::Inherited),
        other => {
            return Err(SageError::config(format!(
                "unsupported profile override '{other}'"
            )));
        }
    };
    Ok(ToolAccessControl::Specific(tools))
}

pub fn validate_tool_names(tools: &[String]) -> SageResult<()> {
    for tool in tools {
        validate_token("tool name", tool)?;
    }
    Ok(())
}

fn validate_non_empty(label: &str, value: &str) -> SageResult<()> {
    if value.trim().is_empty() {
        return Err(SageError::config(format!("{label} is empty")));
    }
    Ok(())
}

fn validate_token(label: &str, value: &str) -> SageResult<()> {
    validate_non_empty(label, value)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(SageError::config(format!(
            "{label} contains unsupported characters"
        )));
    }
    Ok(())
}

fn validate_model_token(value: &str) -> SageResult<()> {
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':'))
    {
        return Err(SageError::config(
            "model override contains unsupported characters",
        ));
    }
    Ok(())
}
