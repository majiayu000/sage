//! Constructor logic for UnifiedExecutor

use crate::agent::ExecutionOptions;
use crate::config::model::Config;
use crate::config::provider::ProviderConfig;
use crate::context::{AutoCompact, AutoCompactConfig};
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::model_capabilities::get_model_capability;
use crate::llm::provider_types::{LlmProvider, TimeoutConfig};
use crate::session::{
    FileSnapshotTracker, JsonlSessionStorage, MessageChainTracker, SessionContext,
};
use crate::tools::executor::ToolExecutor;
use crate::ui::AnimationManager;
use anyhow::Context;
use std::sync::Arc;

use super::{UnifiedExecutor, input_channel};

impl UnifiedExecutor {
    /// Create a new unified executor with default options
    pub fn new(config: Config) -> SageResult<Self> {
        Self::with_options(config, ExecutionOptions::default())
    }

    /// Create a new unified executor with custom options
    pub fn with_options(config: Config, options: ExecutionOptions) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config
            .default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        tracing::info!(
            "Creating unified executor with provider: {}, model: {}",
            provider_name,
            default_params.model
        );

        // Parse provider
        let provider: LlmProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!(
                "Failed to parse provider name '{}' into a valid LLM provider",
                provider_name
            ))?;

        // Create provider config with generous timeout (5 min default)
        let mut provider_config = ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3);

        // Apply custom base_url if configured
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

        // Create input channel based on mode
        let input_channel = input_channel::create_input_channel(&options);

        // Create animation manager
        let animation_manager = AnimationManager::new();

        // Create JSONL storage (optional, can be enabled later)
        let jsonl_storage = JsonlSessionStorage::default_path().ok().map(Arc::new);

        // Get working directory for context
        let working_dir = options
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Create message chain tracker
        let context = SessionContext::new(working_dir);
        let message_tracker = MessageChainTracker::new().with_context(context);

        // Create auto-compact manager with model-specific context window
        let model_capability = get_model_capability(&default_params.model);
        let auto_compact_config = AutoCompactConfig::default()
            .with_max_tokens(model_capability.context_window as usize);
        let auto_compact = AutoCompact::new(auto_compact_config);

        tracing::debug!(
            "Auto-compact initialized with max context: {} tokens",
            model_capability.context_window
        );

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_client,
            tool_executor,
            options,
            input_channel,
            session_recorder: None,
            animation_manager,
            jsonl_storage,
            message_tracker,
            current_session_id: None,
            file_tracker: FileSnapshotTracker::default_tracker(),
            last_summary_msg_count: 0,
            auto_compact,
        })
    }
}
