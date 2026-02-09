//! OpenRouter provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use serde_json::json;
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

        let mut request_body = super::request_builder::build_openai_request_body(
            &self.model_params.model,
            messages,
            tools,
            &self.model_params,
            true,
            false,
        )?;

        // Force Google provider only to avoid Anthropic 403 errors and Bedrock tool_call format issues
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

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("OpenRouter API request failed: {}", e),
                format!(
                    "Failed to send HTTP request to OpenRouter for model: {}",
                    self.model_params.model
                ),
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "OpenRouter").await);
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| super::error_utils::handle_parse_error(e, "OpenRouter"))?;

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
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("OpenRouter API key not provided"))?;

        let url = format!("{}/api/v1/chat/completions", self.config.get_base_url());

        let mut request_body = super::request_builder::build_openai_request_body(
            &self.model_params.model,
            messages,
            tools,
            &self.model_params,
            true,
            true,
        )?;

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

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("OpenRouter streaming request failed: {}", e),
                "Failed to send HTTP request to OpenRouter streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_stream_http_error(response, "OpenRouter").await);
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();
        Ok(super::openai_stream::openai_sse_stream(byte_stream))
    }
}
