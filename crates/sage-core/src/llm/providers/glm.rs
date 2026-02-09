//! GLM (Zhipu AI) provider implementation

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
        if let Some(key) = self.config.get_api_key() {
            request = request.header("x-api-key", key);
        }

        // Add API version header
        request = request.header("anthropic-version", "2023-06-01");

        tracing::debug!(
            "GLM request body: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("GLM API request failed: {}", e),
                format!(
                    "Failed to send HTTP request to GLM (Zhipu AI) API for model: {}",
                    self.model_params.model
                ),
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "GLM").await);
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| super::error_utils::handle_parse_error(e, "GLM"))?;

        tracing::debug!(
            "GLM API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_anthropic(response_json)
    }

    /// GLM streaming chat completion
    /// Uses Anthropic-compatible SSE streaming format
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        let url = format!("{}/v1/messages", self.config.get_base_url());

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_glm(&user_messages)?,
            "stream": true,
        });

        // Add system message
        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        // Add optional parameters - max_tokens is required for GLM streaming
        request_body["max_tokens"] = json!(self.model_params.max_tokens.unwrap_or(4096));

        if let Some(temperature) = self.model_params.temperature {
            // Convert to f64 and round to 2 decimal places to avoid f32 precision issues
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
        let api_key = self.config.get_api_key();
        tracing::info!(
            "GLM streaming: url={}, has_api_key={}, key_preview={}",
            url,
            api_key.is_some(),
            api_key
                .as_ref()
                .map(|k| {
                    if k.len() > 8 {
                        format!("{}...{}", &k[..4], &k[k.len() - 4..])
                    } else {
                        "***".to_string()
                    }
                })
                .unwrap_or_else(|| "NONE".to_string())
        );
        if let Some(key) = api_key {
            request = request.header("x-api-key", key);
        } else {
            tracing::error!("GLM API key is missing! Check your configuration.");
        }

        // Add API version header
        request = request.header("anthropic-version", "2023-06-01");

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("GLM streaming request failed: {}", e),
                "Failed to send HTTP request to GLM streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_stream_http_error(response, "GLM").await);
        }

        let byte_stream = response.bytes_stream();
        Ok(super::anthropic_stream::anthropic_sse_stream(
            byte_stream,
            "GLM",
        ))
    }
}
