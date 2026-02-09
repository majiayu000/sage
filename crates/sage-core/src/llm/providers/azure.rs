//! Azure OpenAI provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use tracing::instrument;

/// Azure OpenAI provider handler
pub struct AzureProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl AzureProvider {
    /// Create a new Azure provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// Azure OpenAI chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("Azure API key not provided"))?;

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.get_base_url(),
            self.model_params.model,
            self.config
                .api_version
                .as_deref()
                .unwrap_or("2025-02-15-preview")
        );

        let request_body = super::request_builder::build_openai_request_body(
            &self.model_params.model,
            messages,
            tools,
            &self.model_params,
            false,
            false,
        )?;

        let request = self
            .http_client
            .post(&url)
            .header("api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Azure API request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Azure API request failed: {}", e),
                format!(
                    "Failed to send HTTP request to Azure OpenAI deployment: {}",
                    self.model_params.model
                ),
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "Azure OpenAI").await);
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| super::error_utils::handle_parse_error(e, "Azure OpenAI"))?;

        tracing::debug!(
            "Azure API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// Azure OpenAI streaming chat completion
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("Azure API key not provided"))?;

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.get_base_url(),
            self.model_params.model,
            self.config
                .api_version
                .as_deref()
                .unwrap_or("2025-02-15-preview")
        );

        let request_body = super::request_builder::build_openai_request_body(
            &self.model_params.model,
            messages,
            tools,
            &self.model_params,
            false,
            true,
        )?;

        let request = self
            .http_client
            .post(&url)
            .header("api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Azure API streaming request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("Azure streaming request failed: {}", e),
                "Failed to send HTTP request to Azure streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(
                super::error_utils::handle_stream_http_error(response, "Azure OpenAI").await,
            );
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();
        Ok(super::openai_stream::openai_sse_stream(byte_stream))
    }
}
