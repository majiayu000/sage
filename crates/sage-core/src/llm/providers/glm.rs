//! GLM (Zhipu AI) provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use anyhow::Context;
use reqwest::Client;
use serde_json::{Value, json};
use tracing::instrument;

/// GLM provider handler
pub struct GlmProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl GlmProvider {
    /// Create a new GLM provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// GLM chat completion - Anthropic compatible format
    /// Uses <https://open.bigmodel.cn/api/anthropic> endpoint
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let url = format!("{}/v1/messages", self.config.get_base_url());

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_glm(&user_messages)?,
        });

        // Add system message
        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            // Convert to f64 and round to 2 decimal places to avoid f32 precision issues
            // f32: 0.7 -> 0.699999988079071, f64: 0.7 -> 0.7
            let rounded_temp = (temperature as f64 * 100.0).round() / 100.0;
            request_body["temperature"] = json!(rounded_temp);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (GLM format - stricter than Anthropic)
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tool_defs = ToolConverter::to_glm(tools)?;
                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication (x-api-key header for Anthropic format)
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version header
        request = request.header("anthropic-version", "2023-06-01");

        tracing::info!(
            "GLM API request tools count: {}, first tool: {:?}",
            request_body["tools"].as_array().map_or(0, |a| a.len()),
            request_body["tools"]
                .as_array()
                .and_then(|a| a.first())
                .map(|t| t["name"].as_str())
        );

        // Debug: Write full request to file for debugging
        let debug_enabled = std::env::var("SAGE_DEBUG_API").is_ok();
        if debug_enabled {
            if let Ok(json_str) = serde_json::to_string_pretty(&request_body) {
                let debug_dir = std::env::var("SAGE_DEBUG_DIR")
                    .unwrap_or_else(|_| "/tmp/sage_debug".to_string());
                let _ = std::fs::create_dir_all(&debug_dir);
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let _ = std::fs::write(
                    format!("{}/glm_request_{}.json", debug_dir, timestamp),
                    &json_str,
                );
                tracing::debug!("Saved GLM request to {}/glm_request_{}.json", debug_dir, timestamp);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("GLM API request failed: {}", e)))
            .context(format!(
                "Failed to send HTTP request to GLM (Zhipu AI) API for model: {}",
                self.model_params.model
            ))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Save error response for debugging
            if debug_enabled {
                let debug_dir = std::env::var("SAGE_DEBUG_DIR")
                    .unwrap_or_else(|_| "/tmp/sage_debug".to_string());
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let error_info = serde_json::json!({
                    "status": status.as_u16(),
                    "error": error_text,
                    "model": self.model_params.model,
                    "timestamp": timestamp.to_string()
                });
                let _ = std::fs::write(
                    format!("{}/glm_error_{}.json", debug_dir, timestamp),
                    serde_json::to_string_pretty(&error_info).unwrap_or_default(),
                );
                tracing::error!("Saved GLM error to {}/glm_error_{}.json", debug_dir, timestamp);
            }

            return Err(SageError::llm(format!(
                "GLM API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse GLM response: {}", e)))
            .context("Failed to deserialize GLM (Zhipu AI) API response as JSON")?;

        tracing::debug!(
            "GLM API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_anthropic(response_json)
    }

    /// GLM streaming chat completion
    /// Uses Anthropic-compatible SSE streaming format
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        use crate::llm::sse_decoder::SseDecoder;
        use crate::llm::streaming::StreamChunk;
        use crate::types::LlmUsage;
        use futures::StreamExt;
        use std::collections::HashMap;

        let url = format!("{}/v1/messages", self.config.get_base_url());

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_glm(&user_messages)?,
            "stream": true,
        });

        // Add system message
        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        // Add optional parameters - max_tokens is required for GLM streaming
        request_body["max_tokens"] = json!(self.model_params.max_tokens.unwrap_or(4096));

        if let Some(temperature) = self.model_params.temperature {
            // Convert to f64 and round to 2 decimal places to avoid f32 precision issues
            let rounded_temp = (temperature as f64 * 100.0).round() / 100.0;
            request_body["temperature"] = json!(rounded_temp);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (GLM format - stricter than Anthropic)
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tool_defs = ToolConverter::to_glm(tools)?;
                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication (x-api-key header for Anthropic format)
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version header
        request = request.header("anthropic-version", "2023-06-01");

        tracing::debug!(
            "GLM API streaming request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("GLM streaming request failed: {}", e)))
            .context("Failed to send HTTP request to GLM streaming API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "GLM streaming API error: {}",
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
                                    // Message started
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

                                    // Extract usage from message_delta
                                    if let Some(usage_data) = data["usage"].as_object() {
                                        let output_tokens = usage_data
                                            .get("output_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);

                                        state.usage = Some(LlmUsage {
                                            prompt_tokens: 0, // Not provided in delta
                                            completion_tokens: output_tokens as u32,
                                            total_tokens: output_tokens as u32,
                                            cost_usd: None,
                                            cache_creation_input_tokens: None,
                                            cache_read_input_tokens: None,
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
                                            "GLM stream error: {}",
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
