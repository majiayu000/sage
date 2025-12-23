//! Sub-agent runner for executing specialized agents
//!
//! This module provides the actual execution logic for sub-agents, replacing
//! the placeholder implementation in the Task tool.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::builtin::{explore_agent, general_purpose_agent, plan_agent};
use super::types::{
    AgentDefinition, AgentProgress, AgentType, ExecutionMetadata, SubAgentConfig, SubAgentResult,
};
use crate::config::model::Config;
use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::client::LLMClient;
use crate::llm::messages::{LLMMessage, MessageRole};
use crate::llm::provider_types::{LLMProvider, TimeoutConfig};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use anyhow::Context;

/// Sub-agent runner that executes agents with filtered tools
pub struct SubAgentRunner {
    /// LLM client for model interactions
    llm_client: LLMClient,
    /// All available tools
    all_tools: Vec<Arc<dyn Tool>>,
    /// Maximum steps per agent execution
    max_steps: usize,
}

impl SubAgentRunner {
    /// Create a new sub-agent runner from configuration
    pub fn from_config(config: &Config, tools: Vec<Arc<dyn Tool>>) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config
            .default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        // Parse provider
        let provider: LLMProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!(
                "Failed to parse provider name '{}' into a valid LLM provider for sub-agent",
                provider_name
            ))?;

        // Create provider config
        let mut provider_config = ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(60))
            .with_max_retries(3);

        // Apply custom base_url if configured
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        // Create model parameters
        let model_params = default_params.to_llm_parameters();

        // Create LLM client
        let llm_client =
            LLMClient::new(provider, provider_config, model_params).context(format!(
                "Failed to create LLM client for sub-agent runner with provider: {}",
                provider_name
            ))?;

        Ok(Self {
            llm_client,
            all_tools: tools,
            max_steps: 20, // Default max steps for subagents
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

    /// Execute a sub-agent with the given configuration
    pub async fn execute(
        &self,
        config: SubAgentConfig,
        cancel: CancellationToken,
    ) -> SageResult<SubAgentResult> {
        let start_time = Instant::now();
        let agent_id = uuid::Uuid::new_v4().to_string();

        // Get agent definition based on type
        let definition = self.get_agent_definition(&config.agent_type);

        // Filter tools based on agent's allowed tools
        let tools = self.filter_tools(&definition);

        tracing::info!(
            "Starting sub-agent execution: type={}, tools={}, prompt_len={}",
            config.agent_type,
            tools.len(),
            config.prompt.len()
        );

        // Build messages
        let mut messages = Vec::new();

        // Add system prompt
        if !definition.system_prompt.is_empty() {
            messages.push(LLMMessage::system(&definition.system_prompt));
        }

        // Add user task with thoroughness context for Explore agents
        let user_message = if config.agent_type == AgentType::Explore {
            format!(
                "{}\n\n**Thoroughness Level**: {}\n{}\n\nTask: {}",
                definition.description,
                config.thoroughness,
                config.thoroughness.description(),
                config.prompt
            )
        } else {
            format!("{}\n\nTask: {}", definition.description, config.prompt)
        };
        messages.push(LLMMessage::user(user_message));

        // Track execution
        let mut progress = AgentProgress::new();
        let mut metadata = ExecutionMetadata::default();

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
            if progress.current_step >= effective_max_steps as u32 {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
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

            progress.next_step();

            // Execute one step
            match self
                .execute_step(&mut messages, &tools, &mut progress, &mut metadata)
                .await?
            {
                StepResult::Continue => continue,
                StepResult::Completed(output) => {
                    let elapsed_ms = start_time.elapsed().as_millis() as u64;
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

    /// Get agent definition for a given type
    fn get_agent_definition(&self, agent_type: &AgentType) -> AgentDefinition {
        match agent_type {
            AgentType::GeneralPurpose => general_purpose_agent(),
            AgentType::Explore => explore_agent(),
            AgentType::Plan => plan_agent(),
            AgentType::Custom => {
                // Default to general purpose for custom
                general_purpose_agent()
            }
        }
    }

    /// Filter tools based on agent definition
    fn filter_tools(&self, definition: &AgentDefinition) -> Vec<Arc<dyn Tool>> {
        self.all_tools
            .iter()
            .filter(|tool| definition.can_use_tool(tool.name()))
            .cloned()
            .collect()
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        messages: &mut Vec<LLMMessage>,
        tools: &[Arc<dyn Tool>],
        progress: &mut AgentProgress,
        metadata: &mut ExecutionMetadata,
    ) -> SageResult<StepResult> {
        // Get tool schemas
        let tool_schemas: Vec<ToolSchema> = tools.iter().map(|t| t.schema()).collect();

        // Call LLM
        let response = self.llm_client.chat(messages, Some(&tool_schemas)).await?;

        // Update token usage
        if let Some(usage) = &response.usage {
            let tokens = (usage.prompt_tokens + usage.completion_tokens) as u64;
            progress.add_tokens(tokens);
            metadata.total_tokens += tokens;
        }

        // Check if there are tool calls
        if !response.tool_calls.is_empty() {
            // Add assistant message with tool calls
            let assistant_msg = LLMMessage {
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

                let result = self.execute_tool_call(call, tools).await;

                // Add tool result message
                let tool_msg = LLMMessage::tool(
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
            let assistant_msg = LLMMessage::assistant(&response.content);
            messages.push(assistant_msg);

            Ok(StepResult::Completed(response.content))
        }
    }

    /// Execute a tool call
    async fn execute_tool_call(&self, call: &ToolCall, tools: &[Arc<dyn Tool>]) -> ToolResult {
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

        // Execute the tool
        tool.execute_with_timing(call).await
    }
}

/// Result of a single step execution
enum StepResult {
    /// Continue to next step
    Continue,
    /// Task completed with final output
    Completed(String),
}

/// Global sub-agent runner instance (set by the main agent)
static GLOBAL_RUNNER: std::sync::OnceLock<Arc<RwLock<Option<SubAgentRunner>>>> =
    std::sync::OnceLock::new();

/// Initialize the global sub-agent runner from configuration
pub fn init_global_runner_from_config(
    config: &Config,
    tools: Vec<Arc<dyn Tool>>,
) -> SageResult<()> {
    let runner = SubAgentRunner::from_config(config, tools)?;
    init_global_runner(runner);
    Ok(())
}

/// Initialize the global sub-agent runner
pub fn init_global_runner(runner: SubAgentRunner) {
    let lock = GLOBAL_RUNNER.get_or_init(|| Arc::new(RwLock::new(None)));
    if let Ok(mut guard) = lock.try_write() {
        *guard = Some(runner);
        tracing::info!("Global sub-agent runner initialized");
    }
}

/// Update tools in the global runner
pub async fn update_global_runner_tools(tools: Vec<Arc<dyn Tool>>) {
    if let Some(lock) = GLOBAL_RUNNER.get() {
        let mut guard = lock.write().await;
        if let Some(runner) = guard.as_mut() {
            runner.update_tools(tools);
            tracing::debug!("Updated global sub-agent runner tools");
        }
    }
}

/// Get the global sub-agent runner
pub fn get_global_runner() -> Option<Arc<RwLock<Option<SubAgentRunner>>>> {
    GLOBAL_RUNNER.get().cloned()
}

/// Execute a sub-agent using the global runner
pub async fn execute_subagent(config: SubAgentConfig) -> SageResult<SubAgentResult> {
    let runner_lock = get_global_runner().ok_or_else(|| {
        SageError::agent(
            "Sub-agent runner not initialized. Call init_global_runner_from_config first.",
        )
    })?;

    let guard = runner_lock.read().await;
    let runner = guard
        .as_ref()
        .ok_or_else(|| SageError::agent("Sub-agent runner not available"))?;

    let cancel = CancellationToken::new();
    runner.execute(config, cancel).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_result() {
        let continue_result = StepResult::Continue;
        let completed_result = StepResult::Completed("Done".to_string());

        match continue_result {
            StepResult::Continue => {}
            _ => panic!("Expected Continue"),
        }

        match completed_result {
            StepResult::Completed(output) => assert_eq!(output, "Done"),
            _ => panic!("Expected Completed"),
        }
    }
}
