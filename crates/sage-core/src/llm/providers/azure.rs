//! Azure OpenAI provider implementation

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

/// Azure OpenAI provider handler
pub struct AzureProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl AzureProvider {
    /// Create a new Azure provider
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

    /// Azure OpenAI chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
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

        let mut request_body = json!({
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
            .header("api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!(
            "Azure API request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Azure API request failed: {}", e)))
            .context(format!(
                "Failed to send HTTP request to Azure OpenAI deployment: {}",
                self.model_params.model
            ))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Azure API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Azure response: {}", e)))
            .context("Failed to deserialize Azure OpenAI API response as JSON")?;

        tracing::debug!(
            "Azure API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// Azure OpenAI streaming chat completion
    /// Note: Streaming not yet implemented for Azure
    pub async fn chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement Azure streaming (similar to OpenAI)
        Err(SageError::llm("Azure streaming not yet implemented"))
    }
}
