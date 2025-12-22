//! Doubao provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::{LLMMessage, LLMResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LLMStream;
use crate::tools::types::ToolSchema;
use anyhow::Context;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::instrument;

/// Doubao provider handler
pub struct DoubaoProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl DoubaoProvider {
    /// Create a new Doubao provider
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

    /// Doubao chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("Doubao API key not provided"))?;

        let url = format!("{}/api/v3/chat/completions", self.config.get_base_url());

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

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Doubao API request failed: {}", e)))
            .context("Failed to send HTTP request to Doubao API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Doubao API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Doubao response: {}", e)))
            .context("Failed to deserialize Doubao API response as JSON")?;

        tracing::debug!(
            "Doubao API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// Doubao streaming chat completion (not yet implemented)
    pub async fn chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement Doubao streaming
        Err(SageError::llm("Doubao streaming not yet implemented"))
    }
}
