//! Anthropic provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::LlmMessage;
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use serde_json::{Value, json};
use tracing::instrument;

/// Anthropic provider handler
pub struct AnthropicProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// Anthropic chat completion
    ///
    /// Supports prompt caching when `enable_prompt_caching` is set in ModelParameters.
    /// When enabled, system prompts and tools are cached for faster subsequent requests.
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<crate::llm::messages::LlmResponse> {
        let url = format!("{}/v1/messages", self.config.get_base_url());
        let enable_caching = self.model_params.is_prompt_caching_enabled();

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_anthropic(&user_messages, enable_caching)?,
        });

        // Add system message with optional cache_control
        if let Some(system) = system_message {
            if enable_caching {
                // Use array format with cache_control for caching
                request_body["system"] = json!([{
                    "type": "text",
                    "text": system,
                    "cache_control": {"type": "ephemeral"}
                }]);
            } else {
                request_body["system"] = json!(system);
            }
        }

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        // Anthropic API doesn't allow both temperature and top_p - use temperature if set
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop_sequences"] = json!(stop);
        }

        // Add tools if provided, with optional cache_control on the last tool
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let mut tool_defs: Vec<Value> = ToolConverter::to_anthropic(tools)?;

                // Add cache_control to the last tool when caching is enabled
                // This caches all tools as a single cache breakpoint
                if enable_caching {
                    if let Some(last_tool) = tool_defs.last_mut() {
                        if let Some(obj) = last_tool.as_object_mut() {
                            obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
                        }
                    }
                }

                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version
        if let Some(version) = &self.config.api_version {
            request = request.header("anthropic-version", version);
        }

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Anthropic request failed: {}", e),
                "Failed to send HTTP request to Anthropic API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "Anthropic").await);
        }

        let response_json: Value = response.json().await.map_err(|e| {
            super::error_utils::handle_parse_error(e, "Anthropic")
        })?;

        ResponseParser::parse_anthropic(response_json)
    }

    /// Anthropic streaming chat completion
    ///
    /// Handles Anthropic's SSE event types:
    /// - message_start: Initial message metadata
    /// - content_block_start: Start of a content block (text or tool_use)
    /// - content_block_delta: Incremental content (text_delta or input_json_delta)
    /// - content_block_stop: End of a content block
    /// - message_delta: Final message metadata (stop_reason, usage)
    /// - message_stop: Stream end marker
    ///
    /// Supports prompt caching when `enable_prompt_caching` is set in ModelParameters.
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        let url = format!("{}/v1/messages", self.config.get_base_url());
        let enable_caching = self.model_params.is_prompt_caching_enabled();

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_anthropic(&user_messages, enable_caching)?,
            "stream": true,
        });

        // Add system message with optional cache_control
        if let Some(system) = system_message {
            if enable_caching {
                request_body["system"] = json!([{
                    "type": "text",
                    "text": system,
                    "cache_control": {"type": "ephemeral"}
                }]);
            } else {
                request_body["system"] = json!(system);
            }
        }

        // Add optional parameters - max_tokens is required for Anthropic
        request_body["max_tokens"] = json!(self.model_params.max_tokens.unwrap_or(4096));

        // Anthropic API doesn't allow both temperature and top_p - use temperature if set
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop_sequences"] = json!(stop);
        }

        // Add tools if provided, with optional cache_control
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let mut tool_defs: Vec<Value> = ToolConverter::to_anthropic(tools)?;

                if enable_caching {
                    if let Some(last_tool) = tool_defs.last_mut() {
                        if let Some(obj) = last_tool.as_object_mut() {
                            obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
                        }
                    }
                }

                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version (required for Anthropic)
        let api_version = self.config.api_version.as_deref().unwrap_or("2023-06-01");
        request = request.header("anthropic-version", api_version);

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Anthropic streaming request failed: {}", e),
                "Failed to send HTTP request to Anthropic streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_stream_http_error(response, "Anthropic").await);
        }

        let byte_stream = response.bytes_stream();
        Ok(super::anthropic_stream::anthropic_sse_stream(
            byte_stream,
            "Anthropic",
        ))
    }
}
