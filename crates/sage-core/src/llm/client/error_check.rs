//! Error checking and classification for retry and fallback logic

use super::types::LlmClient;
use crate::error::SageError;

impl LlmClient {
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
                msg_lower.contains("503")
                    || msg_lower.contains("502")
                    || msg_lower.contains("504")
                    || msg_lower.contains("429")
                    || msg_lower.contains("overloaded")
                    || msg_lower.contains("timeout")
                    || msg_lower.contains("connection")
                    || msg_lower.contains("network")
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
                msg_lower.contains("403")
                    || msg_lower.contains("429")
                    || msg_lower.contains("quota")
                    || msg_lower.contains("rate limit")
                    || msg_lower.contains("insufficient")
                    || msg_lower.contains("exceeded")
                    || msg_lower.contains("not enough")
                    || msg_lower.contains("token quota")
            }
            SageError::Http {
                status_code: Some(code),
                ..
            } => *code == 403 || *code == 429,
            _ => false,
        }
    }
}
