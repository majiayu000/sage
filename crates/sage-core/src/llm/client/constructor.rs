//! LLM client constructor and initialization logic

use super::types::LlmClient;
use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::provider_types::{LlmProvider, ModelParameters};
use crate::llm::providers::{
    AnthropicProvider, AzureProvider, DoubaoProvider, GlmProvider, GoogleProvider, OllamaProvider,
    OpenAiProvider, OpenRouterProvider, ProviderInstance,
};
use crate::recovery::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

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
    /// let config = ProviderConfig::new("openai")
    ///     .with_api_key("your-api-key");
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
        config.validate().map_err(|e| {
            SageError::config_with_context(
                format!("Invalid provider config: {}", e),
                format!(
                    "Validating configuration for provider '{}'",
                    provider.name()
                ),
            )
        })?;

        // Get effective timeout configuration (handles legacy timeout field)
        let timeouts = config.get_effective_timeouts();

        // Create HTTP client with comprehensive timeout configuration
        let mut client_builder = Client::builder()
            .connect_timeout(timeouts.connection_timeout())
            .timeout(timeouts.request_timeout());

        // Add custom headers
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in config.headers() {
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

        let http_client = client_builder.build().map_err(|e| {
            SageError::llm_with_provider(
                format!("Failed to create HTTP client: {}", e),
                provider.name(),
            )
        })?;

        debug!(
            "Created LLM client for provider '{}' with timeouts: connection={}s, request={}s",
            provider.name(),
            timeouts.connection_timeout_secs,
            timeouts.request_timeout_secs
        );

        // Clone config and model_params once for LlmClient storage
        // The originals are moved into the provider instance to avoid repeated cloning
        let stored_config = config.clone();
        let stored_model_params = model_params.clone();

        // Create circuit breaker for this provider
        // LLM calls benefit from a lenient configuration due to natural latency variance
        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: 5,                   // Open after 5 consecutive failures
            success_threshold: 2,                   // Close after 2 successes in half-open
            reset_timeout: Duration::from_secs(30), // Try again after 30s
            window_size: Duration::from_secs(60),   // Count failures in 60s window
            half_open_max_requests: 2,              // Allow 2 test requests in half-open
        };
        let circuit_breaker = Arc::new(CircuitBreaker::with_config(
            format!("llm_{}", provider.name()),
            circuit_breaker_config,
        ));

        debug!(
            "Created circuit breaker for LLM provider '{}'",
            provider.name()
        );

        // Create provider instance based on provider type
        // Move (not clone) config and model_params into the selected provider
        let provider_instance = match &provider {
            LlmProvider::OpenAI => {
                ProviderInstance::OpenAI(OpenAiProvider::new(config, model_params, http_client))
            }
            LlmProvider::Anthropic => ProviderInstance::Anthropic(AnthropicProvider::new(
                config,
                model_params,
                http_client,
            )),
            LlmProvider::Google => {
                ProviderInstance::Google(GoogleProvider::new(config, model_params, http_client))
            }
            LlmProvider::Azure => {
                ProviderInstance::Azure(AzureProvider::new(config, model_params, http_client))
            }
            LlmProvider::OpenRouter => ProviderInstance::OpenRouter(OpenRouterProvider::new(
                config,
                model_params,
                http_client,
            )),
            LlmProvider::Ollama => {
                ProviderInstance::Ollama(OllamaProvider::new(config, model_params, http_client))
            }
            LlmProvider::Doubao => {
                ProviderInstance::Doubao(DoubaoProvider::new(config, model_params, http_client))
            }
            LlmProvider::Glm => {
                ProviderInstance::Glm(GlmProvider::new(config, model_params, http_client))
            }
            LlmProvider::Custom(name) => {
                return Err(SageError::llm_with_provider(
                    "Custom provider not implemented. Consider using OpenRouter or Ollama for custom models.".to_string(),
                    name,
                ));
            }
        };

        Ok(Self {
            provider,
            config: stored_config,
            model_params: stored_model_params,
            provider_instance,
            circuit_breaker,
        })
    }
}
