//! Role resolution helpers for the sub-agent runner.

use super::SubAgentRunner;
use crate::agent::subagent::builtin::{explore_agent, general_purpose_agent, plan_agent};
use crate::agent::subagent::load_custom_role_for_config;
use crate::agent::subagent::types::{
    AgentDefinition, AgentType, RoleResolutionMetadata, SubAgentConfig, SubAgentRoleConfig,
    ToolAccessControl, profile_tool_access, validate_model_override, validate_profile_override,
    validate_reasoning_override,
};
use crate::config::ProviderRegistry;
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::llm::provider_types::LlmProvider;

#[derive(Debug)]
pub(super) struct ResolvedAgentRole {
    pub definition: AgentDefinition,
    pub source: String,
    pub model: Option<String>,
    pub reasoning: Option<String>,
    pub profile: Option<String>,
    pub profile_tool_access: Option<ToolAccessControl>,
}

impl SubAgentRunner {
    pub(super) fn resolve_agent_role(
        &self,
        config: &mut SubAgentConfig,
    ) -> SageResult<ResolvedAgentRole> {
        let (mut definition, source) = if let Some(role_path) = &config.role_path {
            let role = load_custom_role_for_config(config)?.ok_or_else(|| {
                SageError::config(format!(
                    "custom role '{}' could not be loaded",
                    role_path.display()
                ))
            })?;
            apply_role_defaults(config, &role);
            (role_definition(role), "custom_role_file".to_string())
        } else {
            match config.agent_type {
                AgentType::GeneralPurpose => (general_purpose_agent(), "builtin".to_string()),
                AgentType::Explore => (explore_agent(), "builtin".to_string()),
                AgentType::Plan => (plan_agent(), "builtin".to_string()),
                AgentType::Custom => {
                    return Err(SageError::config(
                        "custom subagent roles require a role_path",
                    ));
                }
            }
        };

        if let Some(model) = config.model_override.clone().or_else(|| {
            if source == "custom_role_file" {
                definition.model.clone()
            } else {
                None
            }
        }) {
            validate_model_override(&model)?;
            let model = self.resolve_model_override(&model)?;
            config.model_override = Some(model.clone());
            definition.model = Some(model);
        } else if source == "builtin" {
            definition.model = None;
        }
        if let Some(reasoning) = config
            .reasoning_override
            .clone()
            .or_else(|| definition.reasoning.clone())
        {
            validate_reasoning_override(&reasoning)?;
            config.reasoning_override = Some(reasoning.clone());
            definition.reasoning = Some(reasoning);
        }
        let mut resolved_profile_tools = None;
        if let Some(profile) = config
            .profile_override
            .clone()
            .or_else(|| definition.profile.clone())
        {
            validate_profile_override(&profile)?;
            resolved_profile_tools = Some(profile_tool_access(&profile)?);
            config.profile_override = Some(profile.clone());
            definition.profile = Some(profile);
        }
        config.fork_context.validate()?;
        ensure_custom_role_does_not_escalate(
            &definition.available_tools,
            config.parent_tools.as_deref(),
            resolved_profile_tools.as_ref(),
            &source,
        )?;

        Ok(ResolvedAgentRole {
            model: definition.model.clone(),
            reasoning: definition.reasoning.clone(),
            profile: definition.profile.clone(),
            profile_tool_access: resolved_profile_tools,
            definition,
            source,
        })
    }

    pub(super) fn llm_client_for_role(
        &self,
        model: Option<&str>,
        reasoning: Option<&str>,
    ) -> SageResult<Option<LlmClient>> {
        if reasoning.is_some() && !self.reasoning_effort_supported() {
            return Err(SageError::config(format!(
                "reasoning override is unsupported for provider '{}'",
                self.llm_client.provider()
            )));
        }

        if model.is_none() && reasoning.is_none() {
            return Ok(None);
        };
        if model == Some(self.llm_client.model())
            && reasoning == self.llm_client.model_params().reasoning_effort.as_deref()
        {
            return Ok(None);
        }

        let mut params = self.llm_client.model_params().clone();
        if let Some(model) = model {
            params.model = model.to_string();
        }
        params.reasoning_effort = reasoning.map(str::to_string);
        LlmClient::new(
            self.llm_client.provider().clone(),
            self.llm_client.config().clone(),
            params,
        )
        .map(Some)
    }

    fn reasoning_effort_supported(&self) -> bool {
        use crate::llm::provider_types::LlmProvider;
        matches!(
            self.llm_client.provider(),
            LlmProvider::OpenAI
                | LlmProvider::Zai
                | LlmProvider::Azure
                | LlmProvider::OpenRouter
                | LlmProvider::Doubao
                | LlmProvider::Ollama
                | LlmProvider::Glm
                | LlmProvider::Moonshot
                | LlmProvider::Custom(_)
        )
    }

    fn resolve_model_override(&self, model: &str) -> SageResult<String> {
        let resolved = match (self.llm_client.provider().name(), model) {
            ("anthropic", "haiku") => "claude-haiku-4-5",
            ("anthropic", "sonnet") => "claude-sonnet-4-6",
            ("anthropic", "opus") => "claude-opus-4-7",
            (_, "haiku" | "sonnet" | "opus") => {
                return Err(SageError::config(format!(
                    "model alias '{model}' is unsupported for provider '{}'",
                    self.llm_client.provider()
                )));
            }
            _ => model,
        };
        self.ensure_model_allowed_by_provider(resolved)?;
        Ok(resolved.to_string())
    }

    fn ensure_model_allowed_by_provider(&self, model: &str) -> SageResult<()> {
        if dynamic_model_provider(self.llm_client.provider()) {
            return Ok(());
        }
        let provider_name = self.llm_client.provider().name();
        let registry = ProviderRegistry::with_defaults();
        let Some(provider) = registry
            .embedded_providers()
            .into_iter()
            .find(|provider| provider.id == provider_name)
        else {
            return Ok(());
        };
        if provider.models.is_empty()
            || provider
                .models
                .iter()
                .any(|candidate| candidate.id == model)
        {
            return Ok(());
        }
        Err(SageError::config(format!(
            "model override '{model}' is unsupported for provider '{}'",
            self.llm_client.provider()
        )))
    }

    pub(super) fn initial_messages(
        &self,
        definition: &AgentDefinition,
        config: &SubAgentConfig,
        forked_messages: Vec<LlmMessage>,
    ) -> Vec<LlmMessage> {
        let mut messages = Vec::new();
        if !definition.system_prompt.is_empty() {
            messages.push(LlmMessage::system(&definition.system_prompt));
        }
        messages.extend(forked_messages);
        messages.push(LlmMessage::user(user_task_message(definition, config)));
        messages
    }

    pub(super) fn role_metadata(
        &self,
        definition: &AgentDefinition,
        resolved_role: &ResolvedAgentRole,
        config: &SubAgentConfig,
        forked_messages: usize,
        available_tools: Vec<String>,
    ) -> super::ExecutionMetadata {
        super::ExecutionMetadata {
            role_resolution: Some(Box::new(RoleResolutionMetadata {
                role_name: Some(definition.name.clone()),
                role_source: Some(resolved_role.source.clone()),
                model: Some(
                    resolved_role
                        .model
                        .clone()
                        .unwrap_or_else(|| self.llm_client.model().to_string()),
                ),
                reasoning: resolved_role.reasoning.clone(),
                profile: resolved_role.profile.clone(),
                fork_context: Some(config.fork_context.label()),
                forked_messages,
                available_tools,
            })),
            ..Default::default()
        }
    }
}

fn dynamic_model_provider(provider: &LlmProvider) -> bool {
    matches!(
        provider,
        LlmProvider::Azure | LlmProvider::OpenRouter | LlmProvider::Ollama | LlmProvider::Custom(_)
    )
}

fn user_task_message(definition: &AgentDefinition, config: &SubAgentConfig) -> String {
    if config.agent_type == AgentType::Explore {
        format!(
            "{}\n\n**Thoroughness Level**: {}\n{}\n\nTask: {}",
            definition.description,
            config.thoroughness,
            config.thoroughness.description(),
            config.prompt
        )
    } else {
        format!("{}\n\nTask: {}", definition.description, config.prompt)
    }
}

fn apply_role_defaults(config: &mut SubAgentConfig, role: &SubAgentRoleConfig) {
    if let Some(policy) = &role.working_directory_policy {
        config.working_directory = policy.clone();
    }
    if !config.fork_context_explicit
        && let Some(policy) = &role.fork_context
    {
        config.fork_context = policy.clone();
    }
    if config.model_override.is_none() {
        config.model_override = role.model.clone();
    }
    if config.reasoning_override.is_none() {
        config.reasoning_override = role.reasoning.clone();
    }
    if config.profile_override.is_none() {
        config.profile_override = role.profile.clone();
    }
}

fn ensure_custom_role_does_not_escalate(
    access: &ToolAccessControl,
    parent_tools: Option<&[String]>,
    profile_tools: Option<&ToolAccessControl>,
    source: &str,
) -> SageResult<()> {
    if source != "custom_role_file" {
        return Ok(());
    }
    let declared_tools = match access {
        ToolAccessControl::Specific(tools) | ToolAccessControl::InheritedRestricted(tools) => tools,
        ToolAccessControl::None | ToolAccessControl::Inherited => return Ok(()),
        ToolAccessControl::All => {
            return Err(SageError::config(
                "custom role tool scope cannot declare unrestricted access",
            ));
        }
    };

    for tool in declared_tools {
        if parent_tools
            .map(|tools| !tools.iter().any(|parent_tool| parent_tool == tool))
            .unwrap_or(false)
        {
            return Err(SageError::config(format!(
                "custom role declares tool '{tool}' outside parent tool scope"
            )));
        }
        if profile_tools
            .map(|profile| !profile.allows_tool(tool))
            .unwrap_or(false)
        {
            return Err(SageError::config(format!(
                "custom role declares tool '{tool}' outside profile tool scope"
            )));
        }
    }
    Ok(())
}

fn role_definition(role: SubAgentRoleConfig) -> AgentDefinition {
    let available_tools = role.tool_access();
    AgentDefinition {
        agent_type: AgentType::Custom,
        name: role.name,
        description: role.description,
        available_tools,
        model: role.model,
        reasoning: role.reasoning,
        profile: role.profile,
        system_prompt: role.prompt,
    }
}
