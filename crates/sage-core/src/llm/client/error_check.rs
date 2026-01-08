//! Error checking and classification for retry and fallback logic
//!
//! This module implements error classification for the retry pipeline,
//! following patterns similar to Claude Code's dual-strategy approach:
//! - Throttling detection (429, 503)
//! - Exponential backoff for transient errors
//! - System error detection (network issues)

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
    /// ## HTTP Status Codes (following Claude Code patterns)
    /// - HTTP 408 (Request Timeout)
    /// - HTTP 429 (Too Many Requests) - throttling
    /// - HTTP 500 (Internal Server Error)
    /// - HTTP 502 (Bad Gateway)
    /// - HTTP 503 (Service Unavailable) - throttling
    /// - HTTP 504 (Gateway Timeout)
    /// - HTTP 5xx in general (except 501, 505)
    ///
    /// ## System Errors (network-level issues)
    /// - ETIMEDOUT
    /// - ESOCKETTIMEDOUT
    /// - ECONNREFUSED
    /// - ECONNRESET
    /// - ENOTFOUND (DNS resolution failure)
    ///
    /// ## Message Patterns
    /// - "Overloaded" messages
    /// - Timeout errors
    /// - Connection errors
    /// - Network errors
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

                // HTTP status codes that are retryable
                // Following Claude Code: 5xx (except 501, 505) and 408
                let is_retryable_status = msg_lower.contains("408")
                    || msg_lower.contains("429")
                    || msg_lower.contains("500")
                    || msg_lower.contains("502")
                    || msg_lower.contains("503")
                    || msg_lower.contains("504");

                // System/network errors (like Claude Code's isSystemError)
                let is_system_error = msg_lower.contains("etimedout")
                    || msg_lower.contains("esockettimedout")
                    || msg_lower.contains("econnrefused")
                    || msg_lower.contains("econnreset")
                    || msg_lower.contains("enotfound")
                    || msg_lower.contains("timeout")
                    || msg_lower.contains("connection")
                    || msg_lower.contains("network")
                    || msg_lower.contains("dns");

                // Service-level transient errors
                let is_transient = msg_lower.contains("overloaded")
                    || msg_lower.contains("temporarily unavailable")
                    || msg_lower.contains("try again")
                    || msg_lower.contains("service unavailable");

                is_retryable_status || is_system_error || is_transient
            }
            SageError::Http { status_code, .. } => {
                // HTTP errors are retryable based on status code
                match status_code {
                    Some(code) => Self::is_retryable_http_status(*code),
                    None => true, // Network error without status code
                }
            }
            _ => false,
        }
    }

    /// Check if an HTTP status code is retryable.
    ///
    /// Following Claude Code's pattern:
    /// - 5xx errors are retryable (except 501 Not Implemented, 505 HTTP Version Not Supported)
    /// - 408 Request Timeout is retryable
    /// - 429 Too Many Requests is retryable (throttling)
    pub fn is_retryable_http_status(status_code: u16) -> bool {
        match status_code {
            408 => true,                 // Request Timeout
            429 => true,                 // Too Many Requests (throttling)
            500 => true,                 // Internal Server Error
            501 => false,                // Not Implemented - not retryable
            502 => true,                 // Bad Gateway
            503 => true,                 // Service Unavailable (throttling)
            504 => true,                 // Gateway Timeout
            505 => false,                // HTTP Version Not Supported - not retryable
            code if code >= 500 => true, // Other 5xx errors
            _ => false,
        }
    }

    /// Check if this is a throttling error (429 or 503).
    ///
    /// Throttling errors may include Retry-After header hints.
    /// Currently we use exponential backoff, but this method can be extended
    /// to extract and respect Retry-After headers in the future.
    pub fn is_throttling_error(&self, error: &SageError) -> bool {
        match error {
            SageError::Llm { message: msg, .. } => {
                let msg_lower = msg.to_lowercase();
                msg_lower.contains("429") || msg_lower.contains("503")
            }
            SageError::Http {
                status_code: Some(code),
                ..
            } => *code == 429 || *code == 503,
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
