//! OpenAI provider implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::LlmMessage;
use crate::llm::parsers::ResponseParser;
use crate::llm::provider_types::ModelParameters;
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use reqwest::Client;
use serde_json::{Value, json};
use tracing::instrument;

/// OpenAI provider handler
pub struct OpenAiProvider {
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(config: ProviderConfig, model_params: ModelParameters, http_client: Client) -> Self {
        Self {
            config,
            model_params,
            http_client,
        }
    }

    /// OpenAI chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<crate::llm::messages::LlmResponse> {
        let url = format!("{}/chat/completions", self.config.get_base_url());

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
        if let Some(stop) = &self.model_params.stop {
            request_body["stop"] = json!(stop);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
                if let Some(parallel) = self.model_params.parallel_tool_calls {
                    request_body["parallel_tool_calls"] = json!(parallel);
                }
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.bearer_auth(api_key);
        }

        // Add organization header if provided
        if let Some(org) = self.config.organization() {
            request = request.header("OpenAI-Organization", org);
        }

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("OpenAI request failed: {}", e),
                "Failed to send HTTP request to OpenAI API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_http_error(response, "OpenAI").await);
        }

        let response_json: Value = response.json().await.map_err(|e| {
            super::error_utils::handle_parse_error(e, "OpenAI")
        })?;

        ResponseParser::parse_openai(response_json)
    }

    /// OpenAI streaming chat completion
    pub async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        let url = format!("{}/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_openai(messages)?,
            "stream": true,
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
            if !tools.is_empty() {
                request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await.map_err(|e| {
            SageError::llm_with_context(
                format!("OpenAI streaming request failed: {}", e),
                "Failed to send HTTP request to OpenAI streaming API",
            )
        })?;

        if !response.status().is_success() {
            return Err(super::error_utils::handle_stream_http_error(response, "OpenAI").await);
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();
        Ok(super::openai_stream::openai_sse_stream(byte_stream))
    }
}
