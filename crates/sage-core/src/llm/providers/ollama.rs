//! Ollama provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::LlmRequestParams;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use tracing::instrument;

/// Ollama provider handler
pub struct OllamaProvider {
    config: ProviderConfig,
    model_params: LlmRequestParams,
    http_client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(
        config: ProviderConfig,
        model_params: LlmRequestParams,
        http_client: Client,
    ) -> Self {
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

        let request_body = super::request_builder::build_openai_request_body(
            &self.model_params.model,
            messages,
            tools,
            &self.model_params,
            true,
            false,
        )?;

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

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Ollama API request failed: {}", e),
                format!(
                    "Failed to send HTTP request to Ollama for model: {}",
                    self.model_params.model
                ),
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "Ollama").await);
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| super::error_utils::handle_parse_error(e, "Ollama"))?;

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
        let url = format!("{}/v1/chat/completions", self.config.get_base_url());

        let request_body = super::request_builder::build_openai_request_body(
            &self.model_params.model,
            messages,
            tools,
            &self.model_params,
            true,
            true,
        )?;

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

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Ollama streaming request failed: {}", e),
                "Failed to send HTTP request to Ollama streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_stream_http_error(response, "Ollama").await);
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();
        Ok(super::openai_stream::openai_sse_stream(byte_stream))
    }
}
