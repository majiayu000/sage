//! Core agent structure and initialization

use super::types::{FileOperationTracker, TokenUsage};
use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::llm::provider_types::{LlmProvider, TimeoutConfig};
use crate::tools::batch_executor::BatchToolExecutor;
use crate::types::{Id, TaskMetadata};
use anyhow::Context;
use uuid::Uuid;

/// Claude Code style reactive agent implementation
pub struct ClaudeStyleAgent {
    #[allow(dead_code)]
    pub(super) id: Id,
    pub(super) config: Config,
    pub(super) llm_client: LlmClient,
    pub(super) batch_executor: BatchToolExecutor,
    pub(super) conversation_history: Vec<LlmMessage>,
    /// Token usage tracking
    pub(super) token_usage: TokenUsage,
    /// Current step count
    pub(super) current_step: u32,
    /// File operation tracker for completion verification
    pub(super) file_tracker: FileOperationTracker,
}

impl ClaudeStyleAgent {
    /// Create a new Claude-style agent
    pub fn new(config: Config) -> SageResult<Self> {
        // Initialize LLM client
        let default_params = config
            .default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        let provider: LlmProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!(
                "Failed to parse provider name '{}' into a valid LLM provider",
                provider_name
            ))?;

        let mut provider_config = crate::config::provider::ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(60))
            .with_max_retries(3);

        // Apply custom base_url if configured (for OpenRouter, etc.)
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        let model_params = default_params.to_llm_parameters();
        let llm_client =
            LlmClient::new(provider, provider_config, model_params).context(format!(
                "Failed to create LLM client for provider: {}",
                provider_name
            ))?;

        // Initialize batch tool executor
        let batch_executor = BatchToolExecutor::new();

        Ok(Self {
            id: Uuid::new_v4(),
            config,
            llm_client,
            batch_executor,
            conversation_history: Vec::new(),
            token_usage: TokenUsage::new(),
            current_step: 0,
            file_tracker: FileOperationTracker::new(),
        })
    }

    /// Get tool schemas from the batch executor
    pub fn get_tool_schemas(&self) -> Vec<crate::tools::types::ToolSchema> {
        self.batch_executor.get_tool_schemas()
    }

    /// Register a tool with the batch executor
    pub fn register_tool(&mut self, tool: std::sync::Arc<dyn crate::tools::base::Tool>) {
        self.batch_executor.register_tool(tool);
    }

    /// Register multiple tools with the batch executor
    pub fn register_tools(&mut self, tools: Vec<std::sync::Arc<dyn crate::tools::base::Tool>>) {
        self.batch_executor.register_tools(tools);
    }

    /// Check if we can continue execution (budget and step limits)
    pub(super) fn can_continue(&self) -> Result<(), SageError> {
        // Check step limit (None = unlimited)
        if let Some(max_steps) = self
            .config
            .max_steps
            .filter(|&max| self.current_step >= max)
        {
            return Err(SageError::agent(format!(
                "Max steps ({}) reached. Total tokens used: {} (input: {}, output: {})",
                max_steps,
                self.token_usage.total(),
                self.token_usage.input(),
                self.token_usage.output()
            )));
        }

        // Check token budget
        if self
            .token_usage
            .is_budget_exceeded(self.config.total_token_budget)
        {
            return Err(SageError::agent(format!(
                "Token budget ({}) exceeded. Total tokens used: {} (input: {}, output: {})",
                self.config.total_token_budget.unwrap_or(0),
                self.token_usage.total(),
                self.token_usage.input(),
                self.token_usage.output()
            )));
        }

        Ok(())
    }

    /// Get current token usage
    pub fn get_token_usage(&self) -> (u64, u64, u64) {
        (
            self.token_usage.input(),
            self.token_usage.output(),
            self.token_usage.total(),
        )
    }

    /// Get remaining token budget
    pub fn get_remaining_budget(&self) -> Option<u64> {
        self.token_usage.remaining(self.config.total_token_budget)
    }

    /// Get current step count
    pub fn get_current_step(&self) -> u32 {
        self.current_step
    }

    /// Keep conversation history manageable
    pub(super) fn trim_conversation_history(&mut self) {
        const MAX_HISTORY_LENGTH: usize = 20; // Keep last 20 messages

        if self.conversation_history.len() > MAX_HISTORY_LENGTH {
            let keep_from = self.conversation_history.len() - MAX_HISTORY_LENGTH;
            self.conversation_history = self.conversation_history[keep_from..].to_vec();
        }
    }

    /// Clear conversation history for new request
    pub(super) fn clear_history_if_new_task(&mut self, context: Option<&TaskMetadata>) {
        if context.is_some() {
            self.conversation_history.clear();
        }
    }
}
