//! BaseAgent struct and basic implementation

use crate::config::model::Config;
use crate::error::SageResult;
use crate::llm::client::LlmClient;
use crate::llm::provider_types::{LlmProvider, TimeoutConfig};
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::types::Id;
use crate::ui::AnimationManager;
use anyhow::Context;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::error::SageError;

/// Base agent implementation
pub struct BaseAgent {
    pub(super) id: Id,
    pub(super) config: Config,
    pub(super) llm_client: LlmClient,
    pub(super) tool_executor: ToolExecutor,
    pub(super) trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    pub(super) max_steps: u32,
    pub(super) animation_manager: AnimationManager,
}

impl BaseAgent {
    /// Create a new base agent
    pub fn new(config: Config) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config
            .default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        // Debug logging
        tracing::info!("Creating agent with provider: {}", provider_name);
        tracing::info!("Model: {}", default_params.model);
        tracing::info!("API key set: {}", default_params.api_key.is_some());

        // Parse provider
        let provider: LlmProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!(
                "Failed to parse provider name '{}' into a valid LLM provider",
                provider_name
            ))?;

        tracing::info!("Parsed provider: {:?}", provider);

        // Create provider config
        let mut provider_config = crate::config::provider::ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(60))
            .with_max_retries(3);

        // Apply custom base_url if configured (for OpenRouter, etc.)
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        // Create model parameters
        let model_params = default_params.to_llm_parameters();

        // Create LLM client
        let llm_client =
            LlmClient::new(provider, provider_config, model_params).context(format!(
                "Failed to create LLM client for provider: {}",
                provider_name
            ))?;

        // Create tool executor
        let tool_executor = ToolExecutor::new();

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_client,
            tool_executor,
            trajectory_recorder: None,
            max_steps: u32::MAX, // No limit by default
            animation_manager: AnimationManager::new(),
        })
    }

    /// Set trajectory recorder
    pub fn set_trajectory_recorder(&mut self, recorder: Arc<Mutex<TrajectoryRecorder>>) {
        self.trajectory_recorder = Some(recorder);
    }

    /// Set tool executor
    pub fn set_tool_executor(&mut self, executor: ToolExecutor) {
        self.tool_executor = executor;
    }

    /// Set max steps
    pub fn set_max_steps(&mut self, max_steps: u32) {
        self.max_steps = max_steps;
    }

    /// Get tool schemas from the executor
    pub fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        self.tool_executor.get_tool_schemas()
    }
}
