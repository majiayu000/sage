//! Sub-agent runner for executing specialized agents
//!
//! This module provides the actual execution logic for sub-agents, replacing
//! the placeholder implementation in the Task tool.

#[path = "runner_global.rs"]
mod runner_global;
#[path = "runner_mailbox.rs"]
mod runner_mailbox;
#[path = "runner_roles.rs"]
mod runner_roles;

use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use std::path::{Path, PathBuf};

use super::types::{
    AgentDefinition, AgentProgress, AgentType, ExecutionMetadata, SubAgentConfig, SubAgentResult,
};
use super::{AgentPath, SubAgentGraph};
use crate::agent::unified::UnifiedExecutor;
use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::{LlmMessage, MessageRole};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
pub use runner_global::{
    execute_subagent, get_global_runner, get_global_runner_cwd, init_global_runner,
    init_global_runner_from_config, update_global_runner_cwd, update_global_runner_tools,
};
use runner_mailbox::MailboxRuntime;
pub use runner_mailbox::execute_subagent_with_mailbox;

/// Sub-agent runner that executes agents with filtered tools
pub struct SubAgentRunner {
    /// LLM client for model interactions
    llm_client: LlmClient,
    /// All available tools
    all_tools: Vec<Arc<dyn Tool>>,
    /// Maximum steps per agent execution
    max_steps: usize,
    /// Current working directory (for inheritance)
    working_directory: PathBuf,
}

impl SubAgentRunner {
    /// Create a new sub-agent runner from configuration
    ///
    /// # Arguments
    /// * `config` - The main configuration
    /// * `tools` - Available tools for sub-agents
    /// * `working_directory` - The working directory for file operations (optional, defaults to cwd)
    pub fn from_config(
        config: &Config,
        tools: Vec<Arc<dyn Tool>>,
        working_directory: Option<PathBuf>,
    ) -> SageResult<Self> {
        let (llm_client, _provider_name, _model_name) = LlmClient::from_config(config)?;

        // Resolve working directory
        let cwd = working_directory
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Ok(Self {
            llm_client,
            all_tools: tools,
            max_steps: usize::MAX, // No limit by default
            working_directory: cwd,
        })
    }

    /// Set maximum steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Update the tools available to the runner
    pub fn update_tools(&mut self, tools: Vec<Arc<dyn Tool>>) {
        self.all_tools = tools;
    }

    /// Get the current working directory
    pub fn working_directory(&self) -> &PathBuf {
        &self.working_directory
    }

    /// Update the working directory
    pub fn set_working_directory(&mut self, cwd: PathBuf) {
        self.working_directory = cwd;
    }

    /// Get tool names for inheritance
    pub fn tool_names(&self) -> Vec<String> {
        self.all_tools
            .iter()
            .map(|t| t.name().to_string())
            .collect()
    }

    /// Execute a sub-agent with the given configuration
    ///
    /// This method injects the parent's working directory and tool list into
    /// the config for inheritance resolution.
    pub async fn execute(
        &self,
        config: SubAgentConfig,
        cancel: CancellationToken,
    ) -> SageResult<SubAgentResult> {
        self.execute_inner(config, cancel, None).await
    }

    pub async fn execute_with_mailbox(
        &self,
        config: SubAgentConfig,
        cancel: CancellationToken,
        graph: Arc<SubAgentGraph>,
        agent_path: AgentPath,
    ) -> SageResult<SubAgentResult> {
        self.execute_inner(config, cancel, Some(MailboxRuntime::new(graph, agent_path)))
            .await
    }

    async fn execute_inner(
        &self,
        mut config: SubAgentConfig,
        cancel: CancellationToken,
        mut mailbox: Option<MailboxRuntime>,
    ) -> SageResult<SubAgentResult> {
        let start_time = Instant::now();
        let agent_id = uuid::Uuid::new_v4().to_string();

        // Inject parent context for inheritance
        // If not already set, use the runner's working directory and tools
        if config.parent_cwd.is_none() {
            config.parent_cwd = Some(self.working_directory.clone());
        }
        if config.parent_tools.is_none() && config.agent_type == AgentType::Custom {
            return Err(SageError::config(
                "custom subagent roles require explicit parent tool scope",
            ));
        }
        if config.parent_tools.is_none() {
            config.parent_tools = Some(self.tool_names());
        }

        // Resolve the agent role before cwd/tool/message construction so custom
        // roles can safely contribute declarative defaults.
        let resolved_role = self.resolve_agent_role(&mut config)?;

        // Resolve the effective working directory for this sub-agent
        let effective_cwd = config
            .resolve_working_directory()
            .map_err(|e| SageError::agent(format!("Failed to resolve working directory: {}", e)))?;

        tracing::info!(
            "Sub-agent working directory: {:?} (config: {})",
            effective_cwd,
            config.working_directory
        );

        let definition = resolved_role.definition.clone();

        if !matches!(
            config.fork_context,
            crate::agent::subagent::types::ForkContextPolicy::None
        ) && !config.parent_context_available
        {
            return Err(SageError::config(
                "fork_context all/last_n requires available parent context",
            ));
        }

        // Filter tools based on agent, parent/config and profile scopes.
        let tools = self.filter_tools_with_config(
            &definition,
            &config,
            resolved_role.profile_tool_access.as_ref(),
        );
        let tool_names = tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();
        let forked_messages = config
            .fork_context
            .select_messages(&config.parent_context)
            .map_err(|err| SageError::agent(format!("Failed to fork parent context: {}", err)))?;
        let forked_message_count = forked_messages.len();
        let llm_override = self.llm_client_for_role(
            resolved_role.model.as_deref(),
            resolved_role.reasoning.as_deref(),
        )?;
        let llm_client = llm_override.as_ref().unwrap_or(&self.llm_client);

        tracing::info!(
            "Starting sub-agent execution: type={}, role={}, tools={}, cwd={:?}, prompt_len={}",
            config.agent_type,
            definition.name,
            tools.len(),
            effective_cwd,
            config.prompt.len()
        );

        let mut messages = self.initial_messages(&definition, &config, forked_messages);

        // Track execution
        let mut progress = AgentProgress::new();
        let mut metadata = self.role_metadata(
            &definition,
            &resolved_role,
            &config,
            forked_message_count,
            tool_names,
        );

        // Calculate effective max steps based on agent type and thoroughness
        let effective_max_steps = if config.agent_type == AgentType::Explore {
            config.thoroughness.suggested_max_steps()
        } else {
            self.max_steps
        };

        // Execute steps
        loop {
            // Check cancellation
            if cancel.is_cancelled() {
                return Err(SageError::Cancelled);
            }

            // Check step limit
            if progress.current_step >= u32::try_from(effective_max_steps).unwrap_or(u32::MAX) {
                let elapsed_ms =
                    u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);
                metadata.execution_time_ms = elapsed_ms;

                return Ok(SubAgentResult {
                    agent_id,
                    content: format!(
                        "Task incomplete: maximum steps ({}) reached. Last progress: {} tool uses, {} tokens.",
                        effective_max_steps, progress.tool_use_count, progress.token_count
                    ),
                    metadata,
                });
            }

            if let Some(mailbox) = mailbox.as_mut() {
                self.ingest_mailbox_follow_ups(&mut messages, mailbox)
                    .await?;
            }

            progress.next_step();

            // Execute one step
            match self
                .execute_step(
                    llm_client,
                    &mut messages,
                    &tools,
                    &effective_cwd,
                    &self.working_directory,
                    &mut progress,
                    &mut metadata,
                )
                .await?
            {
                StepResult::Continue => continue,
                StepResult::Completed(output) => {
                    if let Some(mailbox) = mailbox.as_mut()
                        && self
                            .ingest_mailbox_follow_ups(&mut messages, mailbox)
                            .await?
                    {
                        continue;
                    }
                    let elapsed_ms =
                        u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);
                    metadata.execution_time_ms = elapsed_ms;

                    return Ok(SubAgentResult {
                        agent_id,
                        content: output,
                        metadata,
                    });
                }
            }
        }
    }

    /// Filter tools based on both agent definition and config's tool access control
    ///
    /// This method considers:
    /// 1. The agent definition's allowed tools
    /// 2. The config's tool access (which may inherit from parent)
    fn filter_tools_with_config(
        &self,
        definition: &AgentDefinition,
        config: &SubAgentConfig,
        profile_access: Option<&crate::agent::subagent::types::ToolAccessControl>,
    ) -> Vec<Arc<dyn Tool>> {
        self.all_tools
            .iter()
            .filter(|tool| {
                let name = tool.name();
                // Must be allowed by agent definition
                let allowed_by_definition = definition.can_use_tool(name);
                // Must be allowed by config's tool access
                let allowed_by_config = config.allows_tool(name);
                let allowed_by_profile = profile_access
                    .map(|profile| profile.allows_tool(name))
                    .unwrap_or(true);
                allowed_by_definition && allowed_by_config && allowed_by_profile
            })
            .cloned()
            .collect()
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        llm_client: &LlmClient,
        messages: &mut Vec<LlmMessage>,
        tools: &[Arc<dyn Tool>],
        _working_dir: &Path,
        tool_cwd: &Path,
        progress: &mut AgentProgress,
        metadata: &mut ExecutionMetadata,
    ) -> SageResult<StepResult> {
        // Get tool schemas
        let tool_schemas: Vec<ToolSchema> = tools.iter().map(|t| t.schema()).collect();

        // Call LLM
        let response = llm_client.chat(messages, Some(&tool_schemas)).await?;

        // Update token usage
        if let Some(usage) = &response.usage {
            let tokens = usage.total_tokens();
            progress.add_tokens(tokens);
            metadata.total_tokens += tokens;
        }

        // Check if there are tool calls
        if !response.tool_calls.is_empty() {
            // Add assistant message with tool calls
            let assistant_msg = LlmMessage {
                role: MessageRole::Assistant,
                content: response.content.clone(),
                tool_calls: Some(response.tool_calls.clone()),
                tool_call_id: None,
                cache_control: None,
                name: None,
                metadata: Default::default(),
            };
            messages.push(assistant_msg);

            // Execute tool calls
            for call in &response.tool_calls {
                progress.increment_tool_use();
                progress.add_activity(format!("Running tool: {}", call.name));
                metadata.add_tool(call.name.clone());
                metadata.total_tool_uses += 1;

                let result = self.execute_tool_call(call, tools, tool_cwd).await;

                // Add tool result message
                let tool_msg = LlmMessage::tool(
                    result
                        .output
                        .unwrap_or_else(|| result.error.unwrap_or_default()),
                    call.id.clone(),
                    Some(call.name.clone()),
                );
                messages.push(tool_msg);
            }

            Ok(StepResult::Continue)
        } else {
            // No tool calls - this is the final response
            let assistant_msg = LlmMessage::assistant(&response.content);
            messages.push(assistant_msg);

            Ok(StepResult::Completed(response.content))
        }
    }

    /// Execute a tool call
    async fn execute_tool_call(
        &self,
        call: &ToolCall,
        tools: &[Arc<dyn Tool>],
        working_dir: &Path,
    ) -> ToolResult {
        // Find the tool
        let tool = match tools.iter().find(|t| t.name() == call.name) {
            Some(t) => t,
            None => {
                return ToolResult::error(
                    &call.id,
                    &call.name,
                    format!("Tool '{}' not found", call.name),
                );
            }
        };

        if let Some(blocked) = Self::settings_permission_block(call, working_dir) {
            return blocked;
        }

        // Execute the tool
        tool.execute_with_timing(call).await
    }

    pub(super) fn settings_permission_block(
        call: &ToolCall,
        working_dir: &Path,
    ) -> Option<ToolResult> {
        match UnifiedExecutor::unattended_settings_permission_result(call, working_dir) {
            Ok(result) => result,
            Err(err) => Some(ToolResult::error(
                &call.id,
                &call.name,
                format!("Settings permission check failed: {}", err),
            )),
        }
    }
}

/// Result of a single step execution
pub(crate) enum StepResult {
    /// Continue to next step
    Continue,
    /// Task completed with final output
    Completed(String),
}
