//! Retry logic for LLM requests
//!
//! Implements a dual-strategy retry pipeline similar to Claude Code:
//! 1. **Throttling Strategy**: For 429/503 errors, uses longer delays
//! 2. **Exponential Backoff Strategy**: For other transient errors
//!
//! Both strategies use jitter to prevent thundering herd problems.

use super::types::LlmClient;
use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmResponse;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{instrument, warn};

/// Minimum delay for throttling errors (429/503)
const THROTTLING_MIN_DELAY_SECS: u64 = 5;

/// Maximum delay cap for exponential backoff
const MAX_RETRY_DELAY_SECS: u64 = 32;

impl LlmClient {
    /// Execute a request with retry logic and exponential backoff.
    ///
    /// Automatically retries failed requests using a dual-strategy approach:
    ///
    /// # Retry Strategy (following Claude Code patterns)
    ///
    /// ## Throttling Strategy (for 429/503)
    /// - Minimum delay: 5 seconds
    /// - Exponential growth: 5s, 10s, 20s...
    /// - Respects rate limits more conservatively
    ///
    /// ## Exponential Backoff Strategy (for other transient errors)
    /// - Base delay: 2^attempt seconds (1s, 2s, 4s, 8s, ...)
    /// - Jitter: Random value between 0 and delay/2 (like Claude Code)
    /// - Max delay: 32 seconds
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
                        // Calculate delay based on error type (dual strategy)
                        let delay = self.calculate_retry_delay(attempt, &error);

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
                            is_throttling = self.is_throttling_error(&error),
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

    /// Calculate retry delay based on error type (dual strategy)
    ///
    /// - Throttling errors (429/503): Use longer delays starting at 5s
    /// - Other errors: Standard exponential backoff starting at 1s
    ///
    /// Both include jitter (random value between 0 and delay/2) following
    /// Claude Code's pattern for distributed systems.
    fn calculate_retry_delay(&self, attempt: u32, error: &SageError) -> Duration {
        let mut rng = rand::thread_rng();

        if self.is_throttling_error(error) {
            // Throttling strategy: longer delays for rate limit errors
            // Base delay: 5s * 2^attempt = 5s, 10s, 20s...
            let base_delay_secs = THROTTLING_MIN_DELAY_SECS * 2_u64.pow(attempt);
            let capped_delay_secs = base_delay_secs.min(MAX_RETRY_DELAY_SECS);

            // Jitter: random value between delay/2 and delay (like Claude Code)
            let min_delay = capped_delay_secs / 2;
            let jitter_secs = rng.gen_range(0..=(capped_delay_secs - min_delay));

            Duration::from_secs(min_delay + jitter_secs)
        } else {
            // Exponential backoff strategy for transient errors
            // Base delay: 2^attempt seconds = 1s, 2s, 4s, 8s...
            let base_delay_secs = 2_u64.pow(attempt);
            let capped_delay_secs = base_delay_secs.min(MAX_RETRY_DELAY_SECS);

            // Jitter: random value between 0 and delay/2 (like Claude Code)
            let jitter_ms = rng.gen_range(0..=(capped_delay_secs * 500));

            Duration::from_secs(capped_delay_secs) + Duration::from_millis(jitter_ms)
        }
    }
}
