//! OpenAI provider implementation

use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::LLMMessage;
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::{LLMStream, StreamChunk};
use crate::tools::types::ToolSchema;
use crate::config::provider::ProviderConfig;
use anyhow::Context;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use tracing::instrument;

/// OpenAI provider handler
pub struct OpenAIProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(
        config: ProviderConfig,
        model_params: ModelParameters,
        http_client: Client,
    ) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// OpenAI chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<crate::llm::messages::LLMResponse> {
        let url = format!("{}/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_openai(messages)?,
        });

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop"] = json!(stop);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
                if let Some(parallel) = self.model_params.parallel_tool_calls {
                    request_body["parallel_tool_calls"] = json!(parallel);
                }
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.bearer_auth(api_key);
        }

        // Add organization header if provided
        if let Some(org) = &self.config.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("OpenAI request failed: {}", e)))
            .context("Failed to send HTTP request to OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("OpenAI API error (status {}): {}", status, error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse OpenAI response: {}", e)))
            .context("Failed to deserialize OpenAI API response as JSON")?;

        ResponseParser::parse_openai(response_json)
    }

    /// OpenAI streaming chat completion
    pub async fn chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        let url = format!("{}/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_openai(messages)?,
            "stream": true,
        });

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.bearer_auth(api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("OpenAI streaming request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "OpenAI streaming API error: {}",
                error_text
            )));
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();

        let stream = byte_stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    // Convert bytes to string and process lines
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    for line in chunk_str.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            // Remove "data: " prefix
                            if data == "[DONE]" {
                                return Some(Ok(StreamChunk::final_chunk(
                                    None,
                                    Some("stop".to_string()),
                                )));
                            }

                            if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                if let Some(choices) = json_data["choices"].as_array() {
                                    if let Some(choice) = choices.first() {
                                        if let Some(delta) = choice["delta"].as_object() {
                                            if let Some(content) =
                                                delta.get("content").and_then(|v| v.as_str())
                                            {
                                                return Some(Ok(StreamChunk::content(content)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(SageError::llm(format!("Stream error: {}", e)))),
            }
        });

        Ok(Box::pin(stream))
    }
}
