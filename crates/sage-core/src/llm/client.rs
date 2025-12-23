//! LLM client implementation

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::provider_types::{LlmProvider, ModelParameters};
use crate::llm::providers::{
    ProviderInstance, LLMProviderTrait, OpenAIProvider, AnthropicProvider, GoogleProvider,
    AzureProvider, OpenRouterProvider, OllamaProvider, DoubaoProvider, GlmProvider,
};
use crate::llm::rate_limiter::global as rate_limiter;
use crate::llm::streaming::{LlmStream, StreamingLlmClient};
use crate::tools::types::ToolSchema;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, instrument, warn};
use rand::Rng;

/// LLM client for making requests to various providers.
///
/// Provides a unified interface for interacting with multiple LLM providers
/// (OpenAI, Anthropic, Google, Azure, etc.) with automatic retry logic,
/// rate limiting, and streaming support.
///
/// # Features
///
/// - **Multi-provider support**: OpenAI, Anthropic, Google, Azure, OpenRouter, Ollama, Doubao, GLM
/// - **Automatic retries**: Exponential backoff with jitter for transient failures
/// - **Rate limiting**: Global rate limiter prevents hitting API limits
/// - **Streaming**: Support for streaming responses via `StreamingLlmClient` trait
/// - **Timeout control**: Configurable connection and request timeouts
/// - **Custom headers**: Support for custom HTTP headers
///
/// # Examples
///
/// ```no_run
/// use sage_core::llm::client::LlmClient;
/// use sage_core::llm::provider_types::{LlmProvider, ModelParameters};
/// use sage_core::config::provider::ProviderConfig;
/// use sage_core::llm::messages::LlmMessage;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create client for Anthropic
/// let provider = LlmProvider::Anthropic;
/// let config = ProviderConfig::default();
/// let params = ModelParameters {
///     model: "claude-3-5-sonnet-20241022".to_string(),
///     ..Default::default()
/// };
///
/// let client = LlmClient::new(provider, config, params)?;
///
/// // Send a chat request
/// let messages = vec![LlmMessage::user("Hello, world!")];
/// let response = client.chat(&messages, None).await?;
/// println!("Response: {}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct LlmClient {
    provider: LlmProvider,
    config: ProviderConfig,
    model_params: ModelParameters,
    provider_instance: ProviderInstance,
}

/// Deprecated: Use `LlmClient` instead
#[deprecated(since = "0.2.0", note = "Use `LlmClient` instead")]
pub type LLMClient = LlmClient;

impl LlmClient {
    /// Create a new LLM client.
    ///
    /// Initializes the client with provider-specific configuration, validates settings,
    /// and sets up HTTP client with timeout and header configurations.
    ///
    /// # Arguments
    ///
    /// * `provider` - The LLM provider to use
    /// * `config` - Provider-specific configuration (API endpoints, timeouts, etc.)
    /// * `model_params` - Model parameters (model name, temperature, max tokens, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - HTTP client creation fails
    /// - Provider is not implemented (for custom providers)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::{LlmProvider, ModelParameters};
    /// use sage_core::config::provider::ProviderConfig;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = LlmProvider::OpenAI;
    /// let mut config = ProviderConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    ///
    /// let params = ModelParameters {
    ///     model: "gpt-4".to_string(),
    ///     temperature: Some(0.7),
    ///     max_tokens: Some(2000),
    ///     ..Default::default()
    /// };
    ///
    /// let client = LlmClient::new(provider, config, params)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        provider: LlmProvider,
        config: ProviderConfig,
        model_params: ModelParameters,
    ) -> SageResult<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| SageError::config_with_context(
                format!("Invalid provider config: {}", e),
                format!("Validating configuration for provider '{}'", provider.name())
            ))?;

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
            .map_err(|e| SageError::llm_with_provider(
                format!("Failed to create HTTP client: {}", e),
                provider.name()
            ))?;

        debug!(
            "Created LLM client for provider '{}' with timeouts: connection={}s, request={}s",
            provider.name(),
            timeouts.connection_timeout_secs,
            timeouts.request_timeout_secs
        );

        // Create provider instance based on provider type
        let provider_instance = match &provider {
            LlmProvider::OpenAI => ProviderInstance::OpenAI(OpenAIProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Anthropic => ProviderInstance::Anthropic(AnthropicProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Google => ProviderInstance::Google(GoogleProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Azure => ProviderInstance::Azure(AzureProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::OpenRouter => ProviderInstance::OpenRouter(OpenRouterProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Ollama => ProviderInstance::Ollama(OllamaProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Doubao => ProviderInstance::Doubao(DoubaoProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Glm => ProviderInstance::Glm(GlmProvider::new(
                config.clone(),
                model_params.clone(),
                http_client,
            )),
            LlmProvider::Custom(name) => {
                return Err(SageError::llm_with_provider(
                    format!("Custom provider not implemented. Consider using OpenRouter or Ollama for custom models."),
                    name
                ))
            }
        };

        Ok(Self {
            provider,
            config,
            model_params,
            provider_instance,
        })
    }

    /// Get the provider used by this client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// assert!(matches!(client.provider(), LlmProvider::Anthropic));
    /// # Ok(())
    /// # }
    /// ```
    pub fn provider(&self) -> &LlmProvider {
        &self.provider
    }

    /// Get the model name configured for this client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let params = ModelParameters {
    ///     model: "claude-3-5-sonnet-20241022".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     params
    /// )?;
    ///
    /// assert_eq!(client.model(), "claude-3-5-sonnet-20241022");
    /// # Ok(())
    /// # }
    /// ```
    pub fn model(&self) -> &str {
        &self.model_params.model
    }

    /// Get the provider configuration.
    ///
    /// Returns a reference to the provider configuration containing
    /// API endpoints, timeouts, headers, and other settings.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// let config = client.config();
    /// println!("Max retries: {:?}", config.max_retries);
    /// # Ok(())
    /// # }
    /// ```
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Execute a request with retry logic and exponential backoff.
    ///
    /// Automatically retries failed requests using exponential backoff with jitter.
    /// Only retries errors that are likely transient (network issues, rate limits, etc.).
    ///
    /// # Retry Strategy
    ///
    /// - Base delay: 2^attempt seconds (1s, 2s, 4s, 8s, ...)
    /// - Jitter: Random 0-500ms per second of delay
    /// - Max retries: Configured in `ProviderConfig` (default: 3)
    /// - Retryable errors: 429, 502, 503, 504, timeouts, network errors
    ///
    /// # Arguments
    ///
    /// * `operation` - Async closure that performs the LLM request
    ///
    /// # Errors
    ///
    /// Returns the last error if all retry attempts are exhausted.
    /// Non-retryable errors (e.g., 400, 401) return immediately without retrying.
    #[instrument(skip(self, operation), fields(max_retries = %self.config.max_retries.unwrap_or(3)))]
    async fn execute_with_retry<F, Fut>(&self, operation: F) -> SageResult<LlmResponse>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = SageResult<LlmResponse>>,
    {
        let max_retries = self.config.max_retries.unwrap_or(3);
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(response) => {
                    if attempt > 0 {
                        tracing::info!(attempt = attempt, "request succeeded after retry");
                    }
                    if let Some(usage) = &response.usage {
                        tracing::info!(
                            prompt_tokens = usage.prompt_tokens,
                            completion_tokens = usage.completion_tokens,
                            total_tokens = usage.total_tokens,
                            "llm request completed"
                        );
                    }
                    return Ok(response);
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    // Check if error is retryable
                    if !self.is_retryable_error(&error) {
                        warn!("Non-retryable error encountered: {}", error);
                        tracing::warn!(error = %error, "non-retryable error");
                        return Err(error);
                    }

                    if attempt < max_retries {
                        // Calculate exponential backoff with jitter
                        let base_delay_secs = 2_u64.pow(attempt);
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

                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = max_retries + 1,
                            delay_secs = delay.as_secs_f64(),
                            "retrying after failure"
                        );

                        sleep(delay).await;
                    } else {
                        warn!(
                            "Request failed after {} attempts: {}",
                            max_retries + 1,
                            error
                        );
                        tracing::error!(attempts = max_retries + 1, "all retry attempts exhausted");
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            SageError::llm_with_provider(
                format!("All {} retry attempts failed without error details", max_retries + 1),
                self.provider.name()
            )
        }))
    }

    /// Check if an error is retryable.
    ///
    /// Determines whether an error should trigger automatic retry based on
    /// error type and status code.
    ///
    /// # Retryable Errors
    ///
    /// - HTTP 429 (Too Many Requests)
    /// - HTTP 502 (Bad Gateway)
    /// - HTTP 503 (Service Unavailable)
    /// - HTTP 504 (Gateway Timeout)
    /// - Network/connection errors
    /// - Timeout errors
    /// - "Overloaded" messages
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::error::SageError;
    /// # use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// let error = SageError::llm("503 Service Unavailable");
    /// assert!(client.is_retryable_error(&error));
    ///
    /// let error = SageError::llm("401 Unauthorized");
    /// assert!(!client.is_retryable_error(&error));
    /// # Ok(())
    /// # }
    /// ```
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

    /// Check if an error should trigger provider fallback.
    ///
    /// Determines whether the error indicates the provider is unavailable
    /// and a fallback provider should be tried.
    ///
    /// # Fallback Triggers
    ///
    /// - HTTP 403 (Forbidden - typically quota/billing issues)
    /// - HTTP 429 (Rate Limit Exceeded)
    /// - Quota exceeded messages
    /// - Rate limit messages
    /// - Insufficient credit/balance messages
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::error::SageError;
    /// # use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// let error = SageError::llm("429 Rate limit exceeded");
    /// assert!(client.should_fallback_provider(&error));
    ///
    /// let error = SageError::llm("500 Internal Server Error");
    /// assert!(!client.should_fallback_provider(&error));
    /// # Ok(())
    /// # }
    /// ```
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

    /// Send a chat completion request.
    ///
    /// Sends a chat completion request to the configured LLM provider with
    /// automatic retry logic and rate limiting.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history (system, user, assistant messages)
    /// * `tools` - Optional tool schemas for function calling
    ///
    /// # Returns
    ///
    /// Returns the LLM response containing:
    /// - Generated content
    /// - Token usage statistics
    /// - Tool calls (if any)
    /// - Finish reason
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Request fails after all retry attempts
    /// - Provider returns an error response
    /// - Network connectivity issues
    /// - API key is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::messages::LlmMessage;
    /// # use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// let messages = vec![
    ///     LlmMessage::system("You are a helpful assistant."),
    ///     LlmMessage::user("What is the capital of France?"),
    /// ];
    ///
    /// let response = client.chat(&messages, None).await?;
    /// println!("Assistant: {}", response.content);
    /// println!("Tokens used: {:?}", response.usage);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, messages, tools), fields(provider = %self.provider, model = %self.model_params.model))]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
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
impl StreamingLlmClient for LlmClient {
    /// Send a streaming chat completion request.
    ///
    /// Initiates a streaming response from the LLM provider, allowing you to
    /// process generated tokens as they arrive rather than waiting for the
    /// complete response.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history (system, user, assistant messages)
    /// * `tools` - Optional tool schemas for function calling
    ///
    /// # Returns
    ///
    /// Returns an `LlmStream` that yields chunks of the response as they're generated.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Stream initialization fails
    /// - Provider doesn't support streaming
    /// - Network connectivity issues
    /// - API key is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::messages::LlmMessage;
    /// use sage_core::llm::streaming::StreamingLlmClient;
    /// use futures::StreamExt;
    /// # use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// let messages = vec![LlmMessage::user("Tell me a story")];
    /// let mut stream = client.chat_stream(&messages, None).await?;
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(response) => {
    ///             if let Some(content) = response.content {
    ///                 print!("{}", content);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Stream error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, messages, tools), fields(provider = %self.provider, model = %self.model_params.model))]
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
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

        let result = self.provider_instance.chat_stream(messages, tools).await;

        if result.is_ok() {
            tracing::info!("streaming request initiated");
        }

        result
    }
}
