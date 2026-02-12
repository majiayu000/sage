//! LLM client accessor methods

use super::types::LlmClient;
use crate::config::provider::ProviderConfig;
use crate::llm::provider_types::LlmProvider;
use crate::recovery::circuit_breaker::{CircuitBreakerStats, CircuitState};

impl LlmClient {
    /// Get the provider used by this client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::LlmRequestParams;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     LlmRequestParams::default()
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
    /// # use sage_core::llm::provider_types::LlmRequestParams;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let params = LlmRequestParams {
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
    /// # use sage_core::llm::provider_types::LlmRequestParams;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     LlmRequestParams::default()
    /// )?;
    ///
    /// let config = client.config();
    /// println!("Max retries: {:?}", config.max_retries());
    /// # Ok(())
    /// # }
    /// ```
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Get the circuit breaker statistics for monitoring.
    ///
    /// Returns statistics about the circuit breaker state, including
    /// failure counts, success counts, and timing information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::LlmRequestParams;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     LlmRequestParams::default()
    /// )?;
    ///
    /// let stats = client.circuit_breaker_stats().await;
    /// println!("Circuit state: {:?}", stats.state);
    /// println!("Failure count: {}", stats.failure_count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn circuit_breaker_stats(&self) -> CircuitBreakerStats {
        self.circuit_breaker.stats().await
    }

    /// Check if the circuit breaker is currently open (blocking requests).
    ///
    /// Returns `true` if the circuit is open (requests will be rejected),
    /// `false` if the circuit is closed or half-open (requests allowed).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::LlmRequestParams;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     LlmRequestParams::default()
    /// )?;
    ///
    /// if client.is_circuit_open().await {
    ///     println!("Circuit is open, requests will be rejected");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_circuit_open(&self) -> bool {
        self.circuit_breaker.state().await == CircuitState::Open
    }

    /// Manually reset the circuit breaker to closed state.
    ///
    /// This can be useful after fixing an underlying issue to immediately
    /// allow requests again, rather than waiting for the reset timeout.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::LlmRequestParams;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     LlmRequestParams::default()
    /// )?;
    ///
    /// // Reset after fixing an issue
    /// client.reset_circuit_breaker().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset().await
    }
}
