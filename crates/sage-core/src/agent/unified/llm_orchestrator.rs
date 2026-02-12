//! LLM Orchestrator - Centralized LLM communication management
//!
//! This module encapsulates all LLM communication logic including:
//! - Streaming chat completion with real-time display
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
use crate::output::OutputStrategy;
use crate::tools::types::ToolSchema;
use crate::types::TokenUsage;
use anyhow::Context;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::select;
use tokio_stream::StreamExt;
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
        // Use provider-specific API key lookup to get the correct key from env/config
        let api_key_info = default_params.get_api_key_info_for_provider(&provider_name);
        tracing::info!(
            "LLM Orchestrator: provider={}, api_key_source={:?}, has_key={}, key_preview={}",
            provider_name,
            api_key_info.source,
            api_key_info.key.is_some(),
            api_key_info
                .key
                .as_ref()
                .map(|k| {
                    if k.len() > 8 {
                        format!("{}...{}", &k[..4], &k[k.len() - 4..])
                    } else {
                        "***".to_string()
                    }
                })
                .unwrap_or_else(|| "NONE".to_string())
        );
        let mut provider_config = ProviderConfig::new(&provider_name)
            .with_api_key(api_key_info.key.unwrap_or_default())
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
        let client = LlmClient::new(provider, provider_config, model_params).context(format!(
            "Failed to create LLM client for: {}",
            provider_name
        ))?;

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

    /// Execute streaming chat with real-time display to terminal
    ///
    /// This method streams LLM responses in real-time, printing each chunk
    /// as it arrives. This provides a much better user experience compared
    /// to waiting for the complete response.
    ///
    /// # Arguments
    /// * `messages` - Conversation history
    /// * `tools` - Optional tool schemas for function calling
    /// * `cancel_token` - Token to signal cancellation
    /// * `on_first_content` - Optional callback invoked when first content arrives
    ///
    /// # Returns
    /// The complete LLM response
    #[instrument(skip(self, messages, tools, cancel_token, on_first_content), fields(provider = %self.provider_name, model = %self.model_name))]
    pub async fn stream_chat_with_display<F>(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
        cancel_token: CancellationToken,
        on_first_content: Option<F>,
    ) -> SageResult<LlmResponse>
    where
        F: FnOnce(),
    {
        let stream_result = self.client.chat_stream(messages, tools).await;

        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        let mut usage: Option<TokenUsage> = None;
        let mut finish_reason: Option<String> = None;
        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        let mut has_printed_content = false;
        let mut on_first_content = on_first_content;

        loop {
            select! {
                chunk_opt = stream.next() => {
                    match chunk_opt {
                        Some(Ok(chunk)) => {
                            // Print content in real-time
                            if let Some(ref chunk_content) = chunk.content {
                                if !chunk_content.is_empty() {
                                    // Call on_first_content callback on first content
                                    if !has_printed_content {
                                        if let Some(callback) = on_first_content.take() {
                                            callback();
                                        }
                                    }
                                    // Print to terminal immediately
                                    print!("{}", chunk_content);
                                    if io::stdout().flush().is_err() {
                                        tracing::warn!("Failed to flush stdout");
                                    }
                                    content.push_str(chunk_content);
                                    has_printed_content = true;
                                }
                            }

                            // Collect tool calls
                            if let Some(chunk_tool_calls) = chunk.tool_calls {
                                tool_calls.extend(chunk_tool_calls);
                            }

                            // Handle final chunk
                            if chunk.is_final {
                                usage = chunk.usage;
                                finish_reason = chunk.finish_reason;
                            }

                            // Merge metadata
                            for (key, value) in chunk.metadata {
                                metadata.insert(key, value);
                            }
                        }
                        Some(Err(e)) => {
                            if has_printed_content {
                                println!(); // New line after streaming content
                            }
                            return Err(e);
                        }
                        None => {
                            // Stream ended
                            if has_printed_content {
                                println!(); // New line after streaming content
                            }
                            break;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    if has_printed_content {
                        println!(); // New line after streaming content
                    }
                    return Err(SageError::Cancelled);
                }
            }
        }

        Ok(LlmResponse {
            content,
            tool_calls,
            usage,
            model: None,
            finish_reason,
            id: None,
            metadata,
        })
    }

    /// Execute streaming chat with configurable output strategy
    ///
    /// This method uses the Strategy Pattern to allow flexible output handling.
    /// The output strategy determines how content is displayed (streaming, batch, JSON, etc.)
    ///
    /// # Arguments
    /// * `messages` - Conversation history
    /// * `tools` - Optional tool schemas for function calling
    /// * `cancel_token` - Token to signal cancellation
    /// * `output_strategy` - The output strategy to use for display
    ///
    /// # Returns
    /// The complete LLM response
    #[instrument(skip(self, messages, tools, cancel_token, output_strategy), fields(provider = %self.provider_name, model = %self.model_name))]
    pub async fn stream_chat_with_strategy(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
        cancel_token: CancellationToken,
        output_strategy: Arc<dyn OutputStrategy>,
    ) -> SageResult<LlmResponse> {
        // Show thinking indicator while waiting for LLM response
        output_strategy.on_thinking("Thinking...");

        let stream_result = self.client.chat_stream(messages, tools).await;

        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                output_strategy.on_thinking_stop();
                return Err(e);
            }
        };

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        let mut usage: Option<TokenUsage> = None;
        let mut finish_reason: Option<String> = None;
        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        let mut has_content = false;
        let mut thinking_stopped = false;

        loop {
            select! {
                chunk_opt = stream.next() => {
                    match chunk_opt {
                        Some(Ok(chunk)) => {
                            // Handle content via output strategy
                            if let Some(ref chunk_content) = chunk.content {
                                if !chunk_content.is_empty() {
                                    // Stop thinking indicator on first content
                                    if !thinking_stopped {
                                        output_strategy.on_thinking_stop();
                                        thinking_stopped = true;
                                    }
                                    if !has_content {
                                        output_strategy.on_content_start();
                                        has_content = true;
                                    }
                                    output_strategy.on_content_chunk(chunk_content);
                                    content.push_str(chunk_content);
                                }
                            }

                            // Collect tool calls
                            if let Some(chunk_tool_calls) = chunk.tool_calls {
                                tool_calls.extend(chunk_tool_calls);
                            }

                            // Handle final chunk
                            if chunk.is_final {
                                usage = chunk.usage;
                                finish_reason = chunk.finish_reason;
                            }

                            // Merge metadata
                            for (key, value) in chunk.metadata {
                                metadata.insert(key, value);
                            }
                        }
                        Some(Err(e)) => {
                            if !thinking_stopped {
                                output_strategy.on_thinking_stop();
                            }
                            if has_content {
                                output_strategy.on_content_end();
                            }
                            return Err(e);
                        }
                        None => {
                            // Stream ended
                            if !thinking_stopped {
                                output_strategy.on_thinking_stop();
                            }
                            if has_content {
                                output_strategy.on_content_end();
                            }
                            break;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    if !thinking_stopped {
                        output_strategy.on_thinking_stop();
                    }
                    if has_content {
                        output_strategy.on_content_end();
                    }
                    return Err(SageError::Cancelled);
                }
            }
        }

        Ok(LlmResponse {
            content,
            tool_calls,
            usage,
            model: None,
            finish_reason,
            id: None,
            metadata,
        })
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
