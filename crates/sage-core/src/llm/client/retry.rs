//! Retry logic for LLM requests

use super::types::LlmClient;
use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmResponse;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{instrument, warn};

impl LlmClient {
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
    pub(super) async fn execute_with_retry<F, Fut>(&self, operation: F) -> SageResult<LlmResponse>
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
                        let delay =
                            Duration::from_secs(base_delay_secs) + Duration::from_millis(jitter_ms);

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
                format!(
                    "All {} retry attempts failed without error details",
                    max_retries + 1
                ),
                self.provider.name(),
            )
        }))
    }
}
