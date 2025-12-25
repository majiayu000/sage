//! LLM client accessor methods

use super::types::LlmClient;
use crate::config::provider::ProviderConfig;
use crate::llm::provider_types::LlmProvider;

impl LlmClient {
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
}
