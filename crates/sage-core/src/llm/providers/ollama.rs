//! Ollama provider implementation

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

/// Ollama provider handler
pub struct OllamaProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// Ollama chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let url = format!("{}/v1/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_openai(messages)?,
        });

        // Add optional parameters (Ollama supports limited parameters)
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (Ollama has limited tool support)
        if let Some(tools) = tools {
            request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
        }

        let request = self
            .http_client
            .post(&url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.config
                        .get_api_key()
                        .unwrap_or_else(|| "ollama".to_string())
                ),
            )
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Ollama API request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Ollama API request failed: {}", e)))
            .context(format!(
                "Failed to send HTTP request to Ollama for model: {}",
                self.model_params.model
            ))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Ollama API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Ollama response: {}", e)))
            .context("Failed to deserialize Ollama API response as JSON")?;

        tracing::debug!(
            "Ollama API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// Ollama streaming chat completion
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        use crate::llm::streaming::StreamChunk;
        use futures::StreamExt;

        let url = format!("{}/v1/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_openai(messages)?,
            "stream": true,
        });

        // Add optional parameters (Ollama supports limited parameters)
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (Ollama has limited tool support)
        if let Some(tools) = tools {
            request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
        }

        let request = self
            .http_client
            .post(&url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.config
                        .get_api_key()
                        .unwrap_or_else(|| "ollama".to_string())
                ),
            )
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Ollama API streaming request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Ollama streaming request failed: {}", e)))
            .context("Failed to send HTTP request to Ollama streaming API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Ollama streaming API error: {}",
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
