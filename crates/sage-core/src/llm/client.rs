//! LLM client implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::messages::{LLMMessage, LLMResponse};
use crate::llm::provider_types::{LLMProvider, ModelParameters};
use crate::llm::providers::{
    ProviderInstance, LLMProviderTrait, OpenAIProvider, AnthropicProvider, GoogleProvider,
    AzureProvider, OpenRouterProvider, OllamaProvider, DoubaoProvider, GlmProvider,
};
use crate::llm::rate_limiter::global as rate_limiter;
use crate::llm::streaming::{LLMStream, StreamingLLMClient};
use crate::tools::types::ToolSchema;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, instrument, warn};
use rand::Rng;

/// LLM client for making requests to various providers
pub struct LLMClient {
    provider: LLMProvider,
    config: ProviderConfig,
    model_params: ModelParameters,
    provider_instance: ProviderInstance,
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

        // Create provider instance based on provider type
        let provider_instance = match &provider {
            LLMProvider::OpenAI => ProviderInstance::OpenAI(OpenAIProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Anthropic => ProviderInstance::Anthropic(AnthropicProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Google => ProviderInstance::Google(GoogleProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Azure => ProviderInstance::Azure(AzureProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::OpenRouter => ProviderInstance::OpenRouter(OpenRouterProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Ollama => ProviderInstance::Ollama(OllamaProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Doubao => ProviderInstance::Doubao(DoubaoProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Glm => ProviderInstance::Glm(GlmProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LLMProvider::Custom(name) => {
                return Err(SageError::llm(format!(
                    "Custom provider '{name}' not implemented"
                )))
            }
        };

        Ok(Self {
            provider,
            config,
            model_params,
            provider_instance,
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
                        let base_delay_secs = 2_u64.pow(attempt as u32);
                        let jitter_ms = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(0..=(base_delay_secs * 500))
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

        Err(last_error.unwrap_or_else(|| SageError::llm("All retry attempts failed")))
    }

    /// Check if an error is retryable
    pub fn is_retryable_error(&self, error: &SageError) -> bool {
        match error {
            SageError::Llm { message: msg, .. } => {
                let msg_lower = msg.to_lowercase();
                msg_lower.contains("503") ||
                msg_lower.contains("502") ||
                msg_lower.contains("504") ||
                msg_lower.contains("429") ||
                msg_lower.contains("overloaded") ||
                msg_lower.contains("timeout") ||
                msg_lower.contains("connection") ||
                msg_lower.contains("network")
            }
            SageError::Http { .. } => true,
            _ => false,
        }
    }

    /// Check if an error should trigger provider fallback
    pub fn should_fallback_provider(&self, error: &SageError) -> bool {
        match error {
            SageError::Llm { message: msg, .. } => {
                let msg_lower = msg.to_lowercase();
                msg_lower.contains("403") ||
                msg_lower.contains("429") ||
                msg_lower.contains("quota") ||
                msg_lower.contains("rate limit") ||
                msg_lower.contains("insufficient") ||
                msg_lower.contains("exceeded") ||
                msg_lower.contains("not enough") ||
                msg_lower.contains("token quota")
            }
            SageError::Http { status_code: Some(code), .. } => {
                *code == 403 || *code == 429
            }
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
        self.execute_with_retry(|| async {
            self.provider_instance.chat(messages, tools).await
        })
        .await
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

        self.provider_instance.chat_stream(messages, tools).await
    }
}
