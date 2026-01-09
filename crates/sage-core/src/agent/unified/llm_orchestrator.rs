//! LLM Orchestrator - Centralized LLM communication management
//!
//! This module encapsulates all LLM communication logic including:
//! - Streaming chat completion
//! - Cancellation support
//! - Response collection
//!
//! It provides a clean abstraction layer between the agent executor
//! and the underlying LLM client.

use crate::config::model::Config;
use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::provider_types::{LlmProvider, TimeoutConfig};
use crate::llm::streaming::{StreamingLlmClient, stream_utils};
use crate::tools::types::ToolSchema;
use anyhow::Context;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

/// LLM Orchestrator handles all LLM communication
///
/// This component centralizes LLM interaction logic that was previously
/// scattered across the UnifiedExecutor, providing:
/// - Unified streaming with cancellation support
/// - Clean error handling
/// - Separation of concerns
pub struct LlmOrchestrator {
    /// The underlying LLM client
    client: LlmClient,
    /// Provider name for logging
    provider_name: String,
    /// Model name for logging
    model_name: String,
}

impl LlmOrchestrator {
    /// Create a new LLM orchestrator from configuration
    pub fn from_config(config: &Config) -> SageResult<Self> {
        let default_params = config
            .default_model_parameters()
            .context("Failed to retrieve default model parameters")?;
        let provider_name = config.get_default_provider().to_string();

        // Parse provider
        let provider: LlmProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))?;

        // Create provider config with generous timeout (5 min default)
        let mut provider_config = ProviderConfig::new(&provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3);

        // Apply custom base_url if configured
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        // Create model parameters
        let model_params = default_params.to_llm_parameters();
        let model_name = model_params.model.clone();

        // Create LLM client
        let client = LlmClient::new(provider, provider_config, model_params)
            .context(format!("Failed to create LLM client for: {}", provider_name))?;

        Ok(Self {
            client,
            provider_name,
            model_name,
        })
    }

    /// Create a new LLM orchestrator with an existing client
    pub fn with_client(client: LlmClient, provider_name: String, model_name: String) -> Self {
        Self {
            client,
            provider_name,
            model_name,
        }
    }

    /// Execute streaming chat completion with cancellation support
    ///
    /// This method:
    /// 1. Initiates a streaming request to the LLM
    /// 2. Collects chunks into a complete response
    /// 3. Supports early cancellation via the cancellation token
    ///
    /// # Arguments
    /// * `messages` - Conversation history
    /// * `tools` - Optional tool schemas for function calling
    /// * `cancel_token` - Token to signal cancellation
    ///
    /// # Returns
    /// The complete LLM response or an error if cancelled/failed
    #[instrument(skip(self, messages, tools, cancel_token), fields(provider = %self.provider_name, model = %self.model_name))]
    pub async fn stream_chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
        cancel_token: CancellationToken,
    ) -> SageResult<LlmResponse> {
        select! {
            response = async {
                let stream = self.client.chat_stream(messages, tools).await?;
                stream_utils::collect_stream_with_cancel(stream, &cancel_token).await
            } => {
                response
            }
            _ = cancel_token.cancelled() => {
                Err(SageError::agent("Task interrupted during LLM call"))
            }
        }
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Get a reference to the underlying LLM client
    pub fn client(&self) -> &LlmClient {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::provider::ProviderConfig;
    use crate::llm::provider_types::{LlmProvider, ModelParameters};

    #[test]
    fn test_orchestrator_with_client() {
        // Create a minimal LLM client for testing
        let provider = LlmProvider::OpenAI;
        let provider_config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters {
            model: "gpt-4".to_string(),
            ..Default::default()
        };

        let client = LlmClient::new(provider, provider_config, model_params).unwrap();
        let orchestrator =
            LlmOrchestrator::with_client(client, "openai".to_string(), "gpt-4".to_string());

        assert_eq!(orchestrator.provider_name(), "openai");
        assert_eq!(orchestrator.model_name(), "gpt-4");
    }
}
