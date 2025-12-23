//! Google (Gemini) provider implementation

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
use serde_json::{Value, json};
use tracing::instrument;

/// Google (Gemini) provider handler
pub struct GoogleProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl GoogleProvider {
    /// Create a new Google provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// Google (Gemini) chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("Google API key not provided"))?;

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.config.get_base_url(),
            self.model_params.model,
            api_key
        );

        let converted_messages = MessageConverter::to_google(messages)?;
        tracing::debug!("Google API converted messages: {:?}", converted_messages);

        let mut request_body = json!({
            "contents": converted_messages,
        });

        // Add generation config
        let mut generation_config = json!({});
        if let Some(max_tokens) = self.model_params.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            generation_config["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            generation_config["topP"] = json!(top_p);
        }
        if let Some(top_k) = self.model_params.top_k {
            generation_config["topK"] = json!(top_k);
        }
        if let Some(stop) = &self.model_params.stop {
            generation_config["stopSequences"] = json!(stop);
        }

        if generation_config
            .as_object()
            .map_or(false, |obj| !obj.is_empty())
        {
            request_body["generationConfig"] = generation_config;
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!([{
                    "functionDeclarations": ToolConverter::to_google(tools)?
                }]);
            }
        }

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Google request failed: {}", e)))
            .context(format!(
                "Failed to send HTTP request to Google Gemini API for model: {}",
                self.model_params.model
            ))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Google API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Google response: {}", e)))
            .context("Failed to deserialize Google Gemini API response as JSON")?;

        tracing::debug!(
            "Google API response: {}",
            serde_json::to_string_pretty(&response_json)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );

        ResponseParser::parse_google(response_json, &self.model_params.model)
    }

    /// Google streaming chat completion (not yet implemented)
    pub async fn chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        Err(SageError::llm("Google streaming not yet implemented"))
    }
}
