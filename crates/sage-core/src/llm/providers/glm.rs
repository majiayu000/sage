//! GLM (Zhipu AI) provider implementation

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

/// GLM provider handler
pub struct GlmProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl GlmProvider {
    /// Create a new GLM provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// GLM chat completion - Anthropic compatible format
    /// Uses <https://open.bigmodel.cn/api/anthropic> endpoint
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let url = format!("{}/v1/messages", self.config.get_base_url());

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_glm(&user_messages)?,
        });

        // Add system message
        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            // Convert to f64 and round to 2 decimal places to avoid f32 precision issues
            // f32: 0.7 -> 0.699999988079071, f64: 0.7 -> 0.7
            let rounded_temp = (temperature as f64 * 100.0).round() / 100.0;
            request_body["temperature"] = json!(rounded_temp);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (GLM format - stricter than Anthropic)
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tool_defs = ToolConverter::to_glm(tools)?;
                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication (x-api-key header for Anthropic format)
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version header
        request = request.header("anthropic-version", "2023-06-01");

        tracing::info!(
            "GLM API request tools count: {}, first tool: {:?}",
            request_body["tools"].as_array().map_or(0, |a| a.len()),
            request_body["tools"]
                .as_array()
                .and_then(|a| a.first())
                .map(|t| t["name"].as_str())
        );

        // Debug: Write full request to file for debugging (only in debug builds with env var)
        #[cfg(debug_assertions)]
        if std::env::var("SAGE_DEBUG_REQUESTS").is_ok() {
            if let Ok(json_str) = serde_json::to_string_pretty(&request_body) {
                let _ = std::fs::write("/tmp/glm_request.json", &json_str);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("GLM API request failed: {}", e)))
            .context(format!(
                "Failed to send HTTP request to GLM (Zhipu AI) API for model: {}",
                self.model_params.model
            ))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "GLM API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse GLM response: {}", e)))
            .context("Failed to deserialize GLM (Zhipu AI) API response as JSON")?;

        tracing::debug!(
            "GLM API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_anthropic(response_json)
    }

    /// GLM streaming chat completion (not yet implemented)
    pub async fn chat_stream(
        &self,
        _messages: &[LlmMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        // TODO: Implement GLM streaming
        Err(SageError::llm("GLM streaming not yet implemented"))
    }
}
