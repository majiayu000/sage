//! LLM client implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::{LLMMessage, LLMResponse};
use crate::llm::parsers::ResponseParser;
use crate::llm::providers::{LLMProvider, ModelParameters};
use crate::llm::rate_limiter::global as rate_limiter;
use crate::llm::streaming::{LLMStream, StreamChunk, StreamingLLMClient};
use crate::tools::types::ToolSchema;
use crate::types::LLMUsage;
use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, instrument, warn};
use rand::Rng;

/// LLM client for making requests to various providers
pub struct LLMClient {
    provider: LLMProvider,
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl LLMClient {
    /// Create a new LLM client
    pub fn new(
        provider: LLMProvider,
        config: ProviderConfig,
        model_params: ModelParameters,
    ) -> SageResult<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| SageError::config(format!("Invalid provider config: {}", e)))?;

        // Get effective timeout configuration (handles legacy timeout field)
        let timeouts = config.get_effective_timeouts();

        // Create HTTP client with comprehensive timeout configuration
        let mut client_builder = Client::builder()
            .connect_timeout(timeouts.connection_timeout())
            .timeout(timeouts.request_timeout());

        // Add custom headers
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in &config.headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if !headers.is_empty() {
            client_builder = client_builder.default_headers(headers);
        }

        let http_client = client_builder
            .build()
            .map_err(|e| SageError::llm(format!("Failed to create HTTP client: {}", e)))?;

        debug!(
            "Created LLM client for provider '{}' with timeouts: connection={}s, request={}s",
            provider.name(),
            timeouts.connection_timeout_secs,
            timeouts.request_timeout_secs
        );

        Ok(Self {
            provider,
            config,
            model_params,
            http_client,
        })
    }

    /// Get the provider
    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.model_params.model
    }

    /// Get the provider configuration
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Execute a request with retry logic and exponential backoff
    async fn execute_with_retry<F, Fut>(&self, operation: F) -> SageResult<LLMResponse>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = SageResult<LLMResponse>>,
    {
        let max_retries = self.config.max_retries.unwrap_or(3);
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    last_error = Some(error.clone());

                    // Check if error is retryable
                    if !self.is_retryable_error(&error) {
                        warn!("Non-retryable error encountered: {}", error);
                        return Err(error);
                    }

                    if attempt < max_retries {
                        // Calculate exponential backoff with jitter
                        // Base delay: 2^attempt seconds, then add random jitter (0-50% of base)
                        let base_delay_secs = 2_u64.pow(attempt as u32);
                        let jitter_ms = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(0..=(base_delay_secs * 500)) // 0 to 50% jitter in ms
                        };
                        let delay = Duration::from_secs(base_delay_secs)
                            + Duration::from_millis(jitter_ms);

                        warn!(
                            "Request failed (attempt {}/{}): {}. Retrying in {:.2}s...",
                            attempt + 1,
                            max_retries + 1,
                            error,
                            delay.as_secs_f64()
                        );

                        sleep(delay).await;
                    } else {
                        warn!(
                            "Request failed after {} attempts: {}",
                            max_retries + 1,
                            error
                        );
                    }
                }
            }
        }

        // Return the last error if all retries failed
        Err(last_error.unwrap_or_else(|| SageError::llm("All retry attempts failed")))
    }

    /// Check if an error is retryable
    fn is_retryable_error(&self, error: &SageError) -> bool {
        match error {
            SageError::llm(msg) => {
                // Check for specific retryable error patterns
                let msg_lower = msg.to_lowercase();
                msg_lower.contains("503") ||  // Service Unavailable
                msg_lower.contains("502") ||  // Bad Gateway
                msg_lower.contains("504") ||  // Gateway Timeout
                msg_lower.contains("429") ||  // Too Many Requests
                msg_lower.contains("overloaded") ||
                msg_lower.contains("timeout") ||
                msg_lower.contains("connection") ||
                msg_lower.contains("network")
            }
            SageError::http(_) => true, // HTTP errors are generally retryable
            _ => false,
        }
    }

    /// Send a chat completion request
    #[instrument(skip(self, messages, tools), fields(provider = %self.provider, model = %self.model_params.model))]
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        // Apply rate limiting before making the request
        let provider_name = self.provider.name();
        let limiter = rate_limiter::get_rate_limiter(provider_name).await;

        if let Some(wait_duration) = limiter.acquire().await {
            debug!(
                "Rate limited for provider '{}', waited {:.2}s",
                provider_name,
                wait_duration.as_secs_f64()
            );
        }

        // Execute the request with retry logic
        match &self.provider {
            LLMProvider::OpenAI => {
                self.execute_with_retry(|| self.openai_chat(messages, tools))
                    .await
            }
            LLMProvider::Anthropic => {
                self.execute_with_retry(|| self.anthropic_chat(messages, tools))
                    .await
            }
            LLMProvider::Google => {
                self.execute_with_retry(|| self.google_chat(messages, tools))
                    .await
            }
            LLMProvider::Azure => {
                self.execute_with_retry(|| self.azure_chat(messages, tools))
                    .await
            }
            LLMProvider::OpenRouter => {
                self.execute_with_retry(|| self.openrouter_chat(messages, tools))
                    .await
            }
            LLMProvider::Doubao => {
                self.execute_with_retry(|| self.doubao_chat(messages, tools))
                    .await
            }
            LLMProvider::Ollama => {
                self.execute_with_retry(|| self.ollama_chat(messages, tools))
                    .await
            }
            LLMProvider::Glm => {
                self.execute_with_retry(|| self.glm_chat(messages, tools))
                    .await
            }
            LLMProvider::Custom(name) => {
                // TODO: Implement plugin system for custom providers
                // - Add provider plugin API
                // - Support dynamic provider loading
                // - Implement provider validation and security
                Err(SageError::llm(format!(
                    "Custom provider '{name}' not implemented"
                )))
            }
        }
    }

    /// OpenAI chat completion
    #[instrument(skip(self, messages, tools), level = "debug")]
    async fn openai_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
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
        if let Some(org) = &self.config.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("OpenAI request failed: {}", e)))
            .context("Failed to send HTTP request to OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("OpenAI API error (status {}): {}", status, error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse OpenAI response: {}", e)))
            .context("Failed to deserialize OpenAI API response as JSON")?;

        ResponseParser::parse_openai(response_json)
    }

    /// Anthropic chat completion
    ///
    /// Supports prompt caching when `enable_prompt_caching` is set in ModelParameters.
    /// When enabled, system prompts and tools are cached for faster subsequent requests.
    async fn anthropic_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/v1/messages", self.config.get_base_url());
        let enable_caching = self.model_params.is_prompt_caching_enabled();

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_anthropic(&user_messages, enable_caching)?,
        });

        // Add system message with optional cache_control
        if let Some(system) = system_message {
            if enable_caching {
                // Use array format with cache_control for caching
                request_body["system"] = json!([{
                    "type": "text",
                    "text": system,
                    "cache_control": {"type": "ephemeral"}
                }]);
            } else {
                request_body["system"] = json!(system);
            }
        }

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        // Anthropic API doesn't allow both temperature and top_p - use temperature if set
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop_sequences"] = json!(stop);
        }

        // Add tools if provided, with optional cache_control on the last tool
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let mut tool_defs: Vec<Value> = ToolConverter::to_anthropic(tools)?;

                // Add cache_control to the last tool when caching is enabled
                // This caches all tools as a single cache breakpoint
                if enable_caching {
                    if let Some(last_tool) = tool_defs.last_mut() {
                        if let Some(obj) = last_tool.as_object_mut() {
                            obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
                        }
                    }
                }

                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version
        if let Some(version) = &self.config.api_version {
            request = request.header("anthropic-version", version);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Anthropic request failed: {}", e)))
            .context("Failed to send HTTP request to Anthropic API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Anthropic API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Anthropic response: {}", e)))
            .context("Failed to deserialize Anthropic API response as JSON")?;

        ResponseParser::parse_anthropic(response_json)
    }

    /// Azure OpenAI chat completion
    async fn azure_chat(
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
            .context(format!("Failed to send HTTP request to Azure OpenAI deployment: {}", self.model_params.model))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Azure API error (status {}): {}", status, error_text)));
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

    /// OpenRouter chat completion
    async fn openrouter_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let api_key = self
            .config
            .get_api_key()
            .ok_or_else(|| SageError::llm("OpenRouter API key not provided"))?;

        let url = format!("{}/api/v1/chat/completions", self.config.get_base_url());

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

        // Force Google provider only to avoid Anthropic 403 errors and Bedrock tool_call format issues
        // OpenRouter sometimes routes to Anthropic which returns "Request not allowed"
        // Amazon Bedrock has issues with tool_call/tool_result format translation
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

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("OpenRouter API request failed: {}", e)))
            .context(format!("Failed to send HTTP request to OpenRouter for model: {}", self.model_params.model))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "OpenRouter API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse OpenRouter response: {}", e)))
            .context("Failed to deserialize OpenRouter API response as JSON")?;

        tracing::debug!(
            "OpenRouter API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// Doubao chat completion
    async fn doubao_chat(
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
            return Err(SageError::llm(format!("Doubao API error (status {}): {}", status, error_text)));
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

    /// Google (Gemini) chat completion
    async fn google_chat(
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
            .context(format!("Failed to send HTTP request to Google Gemini API for model: {}", self.model_params.model))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Google API error (status {}): {}", status, error_text)));
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

    /// Ollama chat completion
    async fn ollama_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/v1/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_openai(messages)?,
        });

        // Add optional parameters (Ollama supports limited parameters)
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (Ollama has limited tool support)
        if let Some(tools) = tools {
            request_body["tools"] = json!(ToolConverter::to_openai(tools)?);
        }

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

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Ollama API request failed: {}", e)))
            .context(format!("Failed to send HTTP request to Ollama for model: {}", self.model_params.model))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Ollama API error (status {}): {}", status, error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Ollama response: {}", e)))
            .context("Failed to deserialize Ollama API response as JSON")?;

        tracing::debug!(
            "Ollama API response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        ResponseParser::parse_openai(response_json)
    }

    /// GLM (Zhipu AI) chat completion - Anthropic compatible format
    /// Uses https://open.bigmodel.cn/api/anthropic endpoint
    async fn glm_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
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
            let rounded_temp = ((temperature as f64 * 100.0).round() / 100.0) as f64;
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
            .context(format!("Failed to send HTTP request to GLM (Zhipu AI) API for model: {}", self.model_params.model))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("GLM API error (status {}): {}", status, error_text)));
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
}

// Streaming support implementation
#[async_trait]
impl StreamingLLMClient for LLMClient {
    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // Apply rate limiting before making the request
        let provider_name = self.provider.name();
        let limiter = rate_limiter::get_rate_limiter(provider_name).await;

        if let Some(wait_duration) = limiter.acquire().await {
            debug!(
                "Rate limited for provider '{}' (streaming), waited {:.2}s",
                provider_name,
                wait_duration.as_secs_f64()
            );
        }

        match &self.provider {
            LLMProvider::OpenAI => self.openai_chat_stream(messages, tools).await,
            LLMProvider::Anthropic => self.anthropic_chat_stream(messages, tools).await,
            LLMProvider::Google => self.google_chat_stream(messages, tools).await,
            LLMProvider::Azure => self.azure_chat_stream(messages, tools).await,
            LLMProvider::OpenRouter => self.openrouter_chat_stream(messages, tools).await,
            LLMProvider::Doubao => self.doubao_chat_stream(messages, tools).await,
            LLMProvider::Ollama => self.ollama_chat_stream(messages, tools).await,
            LLMProvider::Glm => self.glm_chat_stream(messages, tools).await,
            LLMProvider::Custom(name) => Err(SageError::llm(format!(
                "Streaming not supported for custom provider '{name}'"
            ))),
        }
    }
}

impl LLMClient {
    /// OpenAI streaming chat completion
    async fn openai_chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        use futures::StreamExt;

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

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("OpenAI streaming request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "OpenAI streaming API error: {}",
                error_text
            )));
        }

        // Convert response to stream
        let byte_stream = response.bytes_stream();

        let stream = byte_stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    // Convert bytes to string and process lines
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    for line in chunk_str.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..]; // Remove "data: " prefix
                            if data == "[DONE]" {
                                return Some(Ok(StreamChunk::final_chunk(
                                    None,
                                    Some("stop".to_string()),
                                )));
                            }

                            if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                if let Some(choices) = json_data["choices"].as_array() {
                                    if let Some(choice) = choices.first() {
                                        if let Some(delta) = choice["delta"].as_object() {
                                            if let Some(content) =
                                                delta.get("content").and_then(|v| v.as_str())
                                            {
                                                return Some(Ok(StreamChunk::content(content)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(SageError::llm(format!("Stream error: {}", e)))),
            }
        });

        Ok(Box::pin(stream))
    }

    /// Anthropic streaming chat completion
    ///
    /// Handles Anthropic's SSE event types:
    /// - message_start: Initial message metadata
    /// - content_block_start: Start of a content block (text or tool_use)
    /// - content_block_delta: Incremental content (text_delta or input_json_delta)
    /// - content_block_stop: End of a content block
    /// - message_delta: Final message metadata (stop_reason, usage)
    /// - message_stop: Stream end marker
    ///
    /// Supports prompt caching when `enable_prompt_caching` is set in ModelParameters.
    async fn anthropic_chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        use crate::llm::sse_decoder::SSEDecoder;
        use futures::StreamExt;

        let url = format!("{}/v1/messages", self.config.get_base_url());
        let enable_caching = self.model_params.is_prompt_caching_enabled();

        let (system_message, user_messages) = MessageConverter::extract_system_message(messages);

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": MessageConverter::to_anthropic(&user_messages, enable_caching)?,
            "stream": true,
        });

        // Add system message with optional cache_control
        if let Some(system) = system_message {
            if enable_caching {
                request_body["system"] = json!([{
                    "type": "text",
                    "text": system,
                    "cache_control": {"type": "ephemeral"}
                }]);
            } else {
                request_body["system"] = json!(system);
            }
        }

        // Add optional parameters - max_tokens is required for Anthropic
        request_body["max_tokens"] = json!(self.model_params.max_tokens.unwrap_or(4096));

        // Anthropic API doesn't allow both temperature and top_p - use temperature if set
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        } else if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop_sequences"] = json!(stop);
        }

        // Add tools if provided, with optional cache_control
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let mut tool_defs: Vec<Value> = ToolConverter::to_anthropic(tools)?;

                if enable_caching {
                    if let Some(last_tool) = tool_defs.last_mut() {
                        if let Some(obj) = last_tool.as_object_mut() {
                            obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
                        }
                    }
                }

                request_body["tools"] = json!(tool_defs);
            }
        }

        let mut request = self.http_client.post(&url).json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version (required for Anthropic)
        let api_version = self.config.api_version.as_deref().unwrap_or("2023-06-01");
        request = request.header("anthropic-version", api_version);

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Anthropic streaming request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Anthropic streaming API error: {}",
                error_text
            )));
        }

        // State for accumulating tool calls
        struct StreamState {
            decoder: SSEDecoder,
            // Current content block being built
            current_block_type: Option<String>,
            current_block_id: Option<String>,
            current_tool_name: Option<String>,
            tool_input_buffer: String,
            // Accumulated tool calls to emit
            pending_tool_calls: Vec<crate::tools::types::ToolCall>,
            // Final message info
            stop_reason: Option<String>,
            usage: Option<LLMUsage>,
        }

        let state = std::sync::Arc::new(tokio::sync::Mutex::new(StreamState {
            decoder: SSEDecoder::new(),
            current_block_type: None,
            current_block_id: None,
            current_tool_name: None,
            tool_input_buffer: String::new(),
            pending_tool_calls: Vec::new(),
            stop_reason: None,
            usage: None,
        }));

        let byte_stream = response.bytes_stream();

        let stream = byte_stream.flat_map(move |chunk_result| {
            let state = state.clone();
            futures::stream::once(async move {
                match chunk_result {
                    Ok(chunk) => {
                        let mut state = state.lock().await;
                        let events = state.decoder.feed(&chunk);
                        let mut chunks: Vec<SageResult<StreamChunk>> = Vec::new();

                        for event in events {
                            // Parse the event data as JSON
                            let data: Value = match serde_json::from_str(&event.data) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };

                            let event_type = event
                                .event_type
                                .as_deref()
                                .or_else(|| data["type"].as_str());

                            match event_type {
                                Some("message_start") => {
                                    // Message started, could extract model info
                                }
                                Some("content_block_start") => {
                                    // Start of a content block
                                    let block_type = data["content_block"]["type"].as_str();
                                    state.current_block_type = block_type.map(String::from);

                                    if block_type == Some("tool_use") {
                                        state.current_block_id =
                                            data["content_block"]["id"].as_str().map(String::from);
                                        state.current_tool_name = data["content_block"]["name"]
                                            .as_str()
                                            .map(String::from);
                                        state.tool_input_buffer.clear();
                                    }
                                }
                                Some("content_block_delta") => {
                                    let delta = &data["delta"];

                                    match delta["type"].as_str() {
                                        Some("text_delta") => {
                                            if let Some(text) = delta["text"].as_str() {
                                                if !text.is_empty() {
                                                    chunks.push(Ok(StreamChunk::content(text)));
                                                }
                                            }
                                        }
                                        Some("input_json_delta") => {
                                            // Accumulate tool input JSON
                                            if let Some(partial) = delta["partial_json"].as_str() {
                                                state.tool_input_buffer.push_str(partial);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Some("content_block_stop") => {
                                    // End of content block
                                    if state.current_block_type.as_deref() == Some("tool_use") {
                                        // Parse accumulated JSON and create tool call
                                        let arguments: HashMap<String, Value> =
                                            serde_json::from_str(&state.tool_input_buffer)
                                                .unwrap_or_default();

                                        // Warn if input is empty (likely a proxy issue)
                                        if arguments.is_empty() && !state.tool_input_buffer.is_empty() {
                                            tracing::warn!(
                                                "Failed to parse tool input JSON for '{}': buffer was '{}'",
                                                state.current_tool_name.as_deref().unwrap_or("unknown"),
                                                &state.tool_input_buffer
                                            );
                                        } else if arguments.is_empty() {
                                            tracing::warn!(
                                                "Tool '{}' received empty input - this may indicate a proxy server issue",
                                                state.current_tool_name.as_deref().unwrap_or("unknown")
                                            );
                                        }

                                        let tool_call = crate::tools::types::ToolCall {
                                            id: state.current_block_id.take().unwrap_or_default(),
                                            name: state
                                                .current_tool_name
                                                .take()
                                                .unwrap_or_default(),
                                            arguments,
                                            call_id: None,
                                        };

                                        state.pending_tool_calls.push(tool_call);
                                        state.tool_input_buffer.clear();
                                    }
                                    state.current_block_type = None;
                                }
                                Some("message_delta") => {
                                    // Final message info
                                    if let Some(stop_reason) = data["delta"]["stop_reason"].as_str()
                                    {
                                        state.stop_reason = Some(stop_reason.to_string());
                                    }

                                    // Extract usage from message_delta (includes cache metrics)
                                    if let Some(usage_data) = data["usage"].as_object() {
                                        let output_tokens = usage_data
                                            .get("output_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);

                                        // Parse cache metrics
                                        let cache_creation_input_tokens = usage_data
                                            .get("cache_creation_input_tokens")
                                            .and_then(|v| v.as_u64())
                                            .map(|v| v as u32);
                                        let cache_read_input_tokens = usage_data
                                            .get("cache_read_input_tokens")
                                            .and_then(|v| v.as_u64())
                                            .map(|v| v as u32);

                                        state.usage = Some(LLMUsage {
                                            prompt_tokens: 0, // Not provided in delta
                                            completion_tokens: output_tokens as u32,
                                            total_tokens: output_tokens as u32,
                                            cost_usd: None,
                                            cache_creation_input_tokens,
                                            cache_read_input_tokens,
                                        });
                                    }
                                }
                                Some("message_stop") => {
                                    // Emit pending tool calls if any
                                    if !state.pending_tool_calls.is_empty() {
                                        let tool_calls =
                                            std::mem::take(&mut state.pending_tool_calls);
                                        chunks.push(Ok(StreamChunk::tool_calls(tool_calls)));
                                    }

                                    // Emit final chunk
                                    chunks.push(Ok(StreamChunk::final_chunk(
                                        state.usage.take(),
                                        state.stop_reason.take(),
                                    )));
                                }
                                Some("ping") | Some("error") => {
                                    // Handle ping (keep-alive) or errors
                                    if event_type == Some("error") {
                                        let error_msg = data["error"]["message"]
                                            .as_str()
                                            .unwrap_or("Unknown error");
                                        chunks.push(Err(SageError::llm(format!(
                                            "Anthropic stream error: {}",
                                            error_msg
                                        ))));
                                    }
                                }
                                _ => {
                                    // Unknown event type, ignore
                                }
                            }
                        }

                        futures::stream::iter(chunks)
                    }
                    Err(e) => futures::stream::iter(vec![Err(SageError::llm(format!(
                        "Stream error: {}",
                        e
                    )))]),
                }
            })
        });

        // Flatten the nested stream
        let flattened = stream.flatten();

        Ok(Box::pin(flattened))
    }

    async fn google_chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement Google streaming
        Err(SageError::llm("Google streaming not yet implemented"))
    }

    async fn azure_chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement Azure streaming (similar to OpenAI)
        Err(SageError::llm("Azure streaming not yet implemented"))
    }

    async fn openrouter_chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement OpenRouter streaming
        Err(SageError::llm("OpenRouter streaming not yet implemented"))
    }

    async fn doubao_chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement Doubao streaming
        Err(SageError::llm("Doubao streaming not yet implemented"))
    }

    async fn ollama_chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement Ollama streaming
        Err(SageError::llm("Ollama streaming not yet implemented"))
    }

    async fn glm_chat_stream(
        &self,
        _messages: &[LLMMessage],
        _tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        // TODO: Implement GLM streaming
        Err(SageError::llm("GLM streaming not yet implemented"))
    }
}
