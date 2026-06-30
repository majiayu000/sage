//! Parent-context and role validation for Task spawns.

use std::path::PathBuf;

use sage_core::agent::subagent::{
    AgentType, ForkContextMessage, SubAgentConfig, SubAgentGraph, SubAgentGraphError, Thoroughness,
    load_custom_role_for_config,
};
use sage_core::thread_store::ThreadStoreError;
use sage_core::tools::permission::ToolContext;

use super::spawn_params::{
    TaskParameters, build_subagent_config, parent_context_from_tool_context,
};

#[derive(Debug, Clone)]
pub(super) struct ParentContextResolution {
    pub messages: Option<Vec<ForkContextMessage>>,
    pub source: Option<&'static str>,
}

impl ParentContextResolution {
    fn none() -> Self {
        Self {
            messages: None,
            source: None,
        }
    }

    pub fn count(&self) -> usize {
        self.messages.as_ref().map(Vec::len).unwrap_or(0)
    }
}

pub(super) async fn resolve_parent_context(
    graph: Option<&SubAgentGraph>,
    context: Option<&ToolContext>,
) -> anyhow::Result<ParentContextResolution> {
    resolve_parent_context_inner(graph, context, false).await
}

pub(super) async fn resolve_parent_context_allow_missing_thread(
    graph: Option<&SubAgentGraph>,
    context: Option<&ToolContext>,
) -> anyhow::Result<ParentContextResolution> {
    resolve_parent_context_inner(graph, context, true).await
}

async fn resolve_parent_context_inner(
    graph: Option<&SubAgentGraph>,
    context: Option<&ToolContext>,
    allow_missing_thread: bool,
) -> anyhow::Result<ParentContextResolution> {
    if let Some(context) = context
        && let Some(messages) = parent_context_from_tool_context(context)?
    {
        return Ok(ParentContextResolution {
            messages: Some(messages),
            source: Some("tool_context"),
        });
    }

    if let (Some(graph), Some(context)) = (graph, context)
        && let Some(session_id) = context.session_id.as_deref()
    {
        match graph.fork_context_messages(session_id).await {
            Ok(messages) => {
                return Ok(ParentContextResolution {
                    messages: Some(messages),
                    source: Some("thread_store"),
                });
            }
            Err(SubAgentGraphError::ThreadStore(ThreadStoreError::ThreadNotFound(_)))
                if allow_missing_thread =>
            {
                return Ok(ParentContextResolution::none());
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(ParentContextResolution::none())
}

pub(super) fn prepare_subagent_config(
    params: &TaskParameters,
    agent_type: AgentType,
    thoroughness: Thoroughness,
    parent_context: Option<Vec<ForkContextMessage>>,
    context: Option<&ToolContext>,
) -> anyhow::Result<SubAgentConfig> {
    let mut config = build_subagent_config(params, agent_type, thoroughness, parent_context);
    if config.parent_cwd.is_none() {
        config = config.with_parent_cwd(parent_cwd(context)?);
    }
    if config.parent_tools.is_none()
        && let Some(parent_tools) = context.and_then(parent_tools_from_tool_context)
    {
        config = config.with_parent_tools(parent_tools);
    }
    if config.agent_type == AgentType::Custom && config.parent_tools.is_none() {
        return Err(anyhow::anyhow!(
            "custom subagent roles require explicit parent tool scope"
        ));
    }
    validate_custom_role_file_at_spawn(&config)?;
    Ok(config)
}

fn parent_cwd(context: Option<&ToolContext>) -> anyhow::Result<PathBuf> {
    context
        .map(|context| context.working_directory.clone())
        .map(Ok)
        .unwrap_or_else(|| std::env::current_dir().map_err(anyhow::Error::from))
}

fn validate_custom_role_file_at_spawn(config: &SubAgentConfig) -> anyhow::Result<()> {
    load_custom_role_for_config(config)
        .map(|_| ())
        .map_err(anyhow::Error::from)
}

fn parent_tools_from_tool_context(context: &ToolContext) -> Option<Vec<String>> {
    let tools = context.metadata.get("parent_tools")?.as_array()?;
    Some(
        tools
            .iter()
            .filter_map(|tool| tool.as_str().map(str::to_string))
            .collect(),
    )
    .filter(|tools: &Vec<String>| !tools.is_empty())
}
