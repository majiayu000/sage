//! Doubao provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use tracing::instrument;

/// Doubao provider handler
pub struct DoubaoProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl DoubaoProvider {
    /// Create a new Doubao provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// Doubao chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("Doubao API key not provided"))?;

        let url = format!("{}/api/v3/chat/completions", self.config.get_base_url());

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
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Doubao API request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Doubao API request failed: {}", e),
                "Failed to send HTTP request to Doubao API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "Doubao").await);
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            super::error_utils::handle_parse_error(e, "Doubao")
        })?;

        tracing::debug!(
            "Doubao API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// Doubao streaming chat completion
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("Doubao API key not provided"))?;

        let url = format!("{}/api/v3/chat/completions", self.config.get_base_url());

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
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Doubao API streaming request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Doubao streaming request failed: {}", e),
                "Failed to send HTTP request to Doubao streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_stream_http_error(response, "Doubao").await);
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();
        Ok(super::openai_stream::openai_sse_stream(byte_stream))
    }
}
