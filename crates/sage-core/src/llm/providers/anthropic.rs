//! Anthropic provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::LlmMessage;
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::{LlmStream, StreamChunk};
use crate::tools::types::ToolSchema;
use crate::types::LlmUsage;
use anyhow::Context;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
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

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Anthropic request failed: {}", e)))
            .context("Failed to send HTTP request to Anthropic API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Anthropic API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Anthropic response: {}", e)))
            .context("Failed to deserialize Anthropic API response as JSON")?;

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
        use crate::llm::sse_decoder::SseDecoder;

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

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Anthropic streaming request failed: {}", e)))
            .context("Failed to send HTTP request to Anthropic streaming API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Anthropic streaming API error: {}",
                error_text
            )));
        }

        // State for accumulating tool calls
        struct StreamState {
            decoder: SseDecoder,
            // Current content block being built
            current_block_type: Option<String>,
            current_block_id: Option<String>,
            current_tool_name: Option<String>,
            tool_input_buffer: String,
            // Accumulated tool calls to emit
            pending_tool_calls: Vec<crate::tools::types::ToolCall>,
            // Final message info
            stop_reason: Option<String>,
            usage: Option<LlmUsage>,
        }

        let state = std::sync::Arc::new(tokio::sync::Mutex::new(StreamState {
            decoder: SseDecoder::new(),
            current_block_type: None,
            current_block_id: None,
            current_tool_name: None,
            tool_input_buffer: String::new(),
            pending_tool_calls: Vec::new(),
            stop_reason: None,
            usage: None,
        }));

        let byte_stream = response.bytes_stream();

        let stream = byte_stream.flat_map(move |chunk_result| {
            let state = state.clone();
            futures::stream::once(async move {
                match chunk_result {
                    Ok(chunk) => {
                        let mut state = state.lock().await;
                        let events = state.decoder.feed(&chunk);
                        let mut chunks: Vec<SageResult<StreamChunk>> = Vec::new();

                        for event in events {
                            // Parse the event data as JSON
                            let data: Value = match serde_json::from_str(&event.data) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };

                            let event_type = event
                                .event_type
                                .as_deref()
                                .or_else(|| data["type"].as_str());

                            match event_type {
                                Some("message_start") => {
                                    // Message started, could extract model info
                                }
                                Some("content_block_start") => {
                                    // Start of a content block
                                    let block_type = data["content_block"]["type"].as_str();
                                    state.current_block_type = block_type.map(String::from);

                                    if block_type == Some("tool_use") {
                                        state.current_block_id =
                                            data["content_block"]["id"].as_str().map(String::from);
                                        state.current_tool_name = data["content_block"]["name"]
                                            .as_str()
                                            .map(String::from);
                                        state.tool_input_buffer.clear();
                                    }
                                }
                                Some("content_block_delta") => {
                                    let delta = &data["delta"];

                                    match delta["type"].as_str() {
                                        Some("text_delta") => {
                                            if let Some(text) = delta["text"].as_str() {
                                                if !text.is_empty() {
                                                    chunks.push(Ok(StreamChunk::content(text)));
                                                }
                                            }
                                        }
                                        Some("input_json_delta") => {
                                            // Accumulate tool input JSON
                                            if let Some(partial) = delta["partial_json"].as_str() {
                                                state.tool_input_buffer.push_str(partial);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Some("content_block_stop") => {
                                    // End of content block
                                    if state.current_block_type.as_deref() == Some("tool_use") {
                                        // Parse accumulated JSON and create tool call
                                        let arguments: HashMap<String, Value> =
                                            serde_json::from_str(&state.tool_input_buffer)
                                                .unwrap_or_default();

                                        // Warn if input is empty (likely a proxy issue)
                                        if arguments.is_empty() && !state.tool_input_buffer.is_empty() {
                                            tracing::warn!(
                                                "Failed to parse tool input JSON for '{}': buffer was '{}'",
                                                state.current_tool_name.as_deref().unwrap_or("unknown"),
                                                &state.tool_input_buffer
                                            );
                                        } else if arguments.is_empty() {
                                            tracing::warn!(
                                                "Tool '{}' received empty input - this may indicate a proxy server issue",
                                                state.current_tool_name.as_deref().unwrap_or("unknown")
                                            );
                                        }

                                        let tool_call = crate::tools::types::ToolCall {
                                            id: state.current_block_id.take().unwrap_or_default(),
                                            name: state
                                                .current_tool_name
                                                .take()
                                                .unwrap_or_default(),
                                            arguments,
                                            call_id: None,
                                        };

                                        state.pending_tool_calls.push(tool_call);
                                        state.tool_input_buffer.clear();
                                    }
                                    state.current_block_type = None;
                                }
                                Some("message_delta") => {
                                    // Final message info
                                    if let Some(stop_reason) = data["delta"]["stop_reason"].as_str()
                                    {
                                        state.stop_reason = Some(stop_reason.to_string());
                                    }

                                    // Extract usage from message_delta (includes cache metrics)
                                    if let Some(usage_data) = data["usage"].as_object() {
                                        let output_tokens = usage_data
                                            .get("output_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);

                                        // Parse cache metrics
                                        let cache_creation_input_tokens = usage_data
                                            .get("cache_creation_input_tokens")
                                            .and_then(|v| v.as_u64())
                                            .map(|v| v as u32);
                                        let cache_read_input_tokens = usage_data
                                            .get("cache_read_input_tokens")
                                            .and_then(|v| v.as_u64())
                                            .map(|v| v as u32);

                                        state.usage = Some(LlmUsage {
                                            prompt_tokens: 0, // Not provided in delta
                                            completion_tokens: output_tokens as u32,
                                            total_tokens: output_tokens as u32,
                                            cost_usd: None,
                                            cache_creation_input_tokens,
                                            cache_read_input_tokens,
                                        });
                                    }
                                }
                                Some("message_stop") => {
                                    // Emit pending tool calls if any
                                    if !state.pending_tool_calls.is_empty() {
                                        let tool_calls =
                                            std::mem::take(&mut state.pending_tool_calls);
                                        chunks.push(Ok(StreamChunk::tool_calls(tool_calls)));
                                    }

                                    // Emit final chunk
                                    chunks.push(Ok(StreamChunk::final_chunk(
                                        state.usage.take(),
                                        state.stop_reason.take(),
                                    )));
                                }
                                Some("ping") | Some("error") => {
                                    // Handle ping (keep-alive) or errors
                                    if event_type == Some("error") {
                                        let error_msg = data["error"]["message"]
                                            .as_str()
                                            .unwrap_or("Unknown error");
                                        chunks.push(Err(SageError::llm(format!(
                                            "Anthropic stream error: {}",
                                            error_msg
                                        ))));
                                    }
                                }
                                _ => {
                                    // Unknown event type, ignore
                                }
                            }
                        }

                        futures::stream::iter(chunks)
                    }
                    Err(e) => futures::stream::iter(vec![Err(SageError::llm(format!(
                        "Stream error: {}",
                        e
                    )))]),
                }
            })
        });

        // Flatten the nested stream
        let flattened = stream.flatten();

        Ok(Box::pin(flattened))
    }
}
