//! OpenRouter provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use serde_json::{Value, json};
use tracing::instrument;

/// OpenRouter provider handler
pub struct OpenRouterProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// OpenRouter chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("OpenRouter API key not provided"))?;

        let url = format!("{}/api/v1/chat/completions", self.config.get_base_url());

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

        // Add tools if provided
        if let Some(tools) = tools {
            request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
        }

        // Force Google provider only to avoid Anthropic 403 errors and Bedrock tool_call format issues
        // OpenRouter sometimes routes to Anthropic which returns "Request not allowed"
        // Amazon Bedrock has issues with tool_call/tool_result format translation
        request_body["provider"] = json!({
            "order": ["Google"],
            "allow_fallbacks": false
        });

        // Log the full request body for debugging tool_call issues
        tracing::info!(
            "OpenRouter API request messages: {}",
            serde_json::to_string_pretty(&request_body["messages"]).unwrap_or_default()
        );

        let request = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "OpenRouter API request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm_with_context(
                format!("OpenRouter API request failed: {}", e),
                format!("Failed to send HTTP request to OpenRouter for model: {}", self.model_params.model),
            ))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "OpenRouter API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm_with_context(
                format!("Failed to parse OpenRouter response: {}", e),
                "Failed to deserialize OpenRouter API response as JSON",
            ))?;

        tracing::debug!(
            "OpenRouter API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// OpenRouter streaming chat completion
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        use crate::llm::streaming::StreamChunk;
        use futures::StreamExt;

        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("OpenRouter API key not provided"))?;

        let url = format!("{}/api/v1/chat/completions", self.config.get_base_url());

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
            request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
        }

        // Force Google provider only (same as non-streaming chat)
        request_body["provider"] = json!({
            "order": ["Google"],
            "allow_fallbacks": false
        });

        let request = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "OpenRouter API streaming request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm_with_context(
                format!("OpenRouter streaming request failed: {}", e),
                "Failed to send HTTP request to OpenRouter streaming API",
            ))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "OpenRouter streaming API error: {}",
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
