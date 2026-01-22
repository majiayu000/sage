//! Retry policies and utilities for handling transient failures
//!
//! Provides configurable retry behavior with backoff strategies.

use super::backoff::{BackoffConfig, BackoffStrategy, ExponentialBackoff};
use super::{ErrorClass, RecoverableError, RecoveryError, classify_error};
use crate::error::SageError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Unified configuration for retry behavior
///
/// This configuration combines retry policy settings with backoff timing configuration.
/// It supports both high-level retry decisions (what to retry) and timing details (how to retry).
///
/// # Example
/// ```
/// use sage_core::recovery::RetryConfig;
/// use std::time::Duration;
///
/// let config = RetryConfig::default()
///     .with_max_attempts(5)
///     .with_initial_delay(Duration::from_millis(200))
///     .with_max_delay(Duration::from_secs(10));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Maximum total time to spend retrying
    #[serde(with = "humantime_serde")]
    pub max_duration: Duration,
    /// Whether to retry on unknown errors
    pub retry_unknown: bool,
    /// Specific error messages to always retry
    #[serde(default)]
    pub retry_on_messages: Vec<String>,
    /// Specific error messages to never retry
    #[serde(default)]
    pub no_retry_on_messages: Vec<String>,
    /// Initial delay before first retry
    #[serde(with = "humantime_serde")]
    pub initial_delay: Duration,
    /// Maximum delay between retries
    #[serde(with = "humantime_serde")]
    pub max_delay: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Add random jitter to prevent thundering herd
    #[serde(default = "default_jitter")]
    pub jitter: bool,
}

fn default_jitter() -> bool {
    true
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            max_duration: Duration::from_secs(300),
            retry_unknown: true,
            retry_on_messages: vec![
                "rate limit".to_string(),
                "timeout".to_string(),
                "overloaded".to_string(),
                "temporarily unavailable".to_string(),
            ],
            no_retry_on_messages: vec![
                "invalid".to_string(),
                "not found".to_string(),
                "unauthorized".to_string(),
                "forbidden".to_string(),
            ],
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new RetryConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config for aggressive retrying
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            max_duration: Duration::from_secs(600),
            retry_unknown: true,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 1.5,
            ..Default::default()
        }
    }

    /// Create a config with limited retries
    pub fn limited(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Create a config that never retries
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            max_duration: Duration::ZERO,
            retry_unknown: false,
            retry_on_messages: vec![],
            no_retry_on_messages: vec![],
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }

    /// Create a config optimized for storage operations
    pub fn for_storage() -> Self {
        Self {
            max_attempts: 3,
            max_duration: Duration::from_secs(30),
            retry_unknown: true,
            retry_on_messages: vec![
                "connection".to_string(),
                "timeout".to_string(),
                "busy".to_string(),
            ],
            no_retry_on_messages: vec![
                "constraint".to_string(),
                "syntax".to_string(),
            ],
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a config optimized for rate-limited APIs
    pub fn for_rate_limited() -> Self {
        Self {
            max_attempts: 5,
            max_duration: Duration::from_secs(600),
            retry_unknown: false,
            retry_on_messages: vec![
                "rate limit".to_string(),
                "429".to_string(),
                "too many requests".to_string(),
            ],
            no_retry_on_messages: vec![],
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Set max attempts
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Set max duration
    pub fn with_max_duration(mut self, duration: Duration) -> Self {
        self.max_duration = duration;
        self
    }

    /// Set initial delay before first retry
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay between retries
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Set whether to retry unknown errors
    pub fn with_retry_unknown(mut self, retry: bool) -> Self {
        self.retry_unknown = retry;
        self
    }

    /// Add messages that should always trigger retry
    pub fn with_retry_on_messages(mut self, messages: Vec<String>) -> Self {
        self.retry_on_messages = messages;
        self
    }

    /// Add messages that should never trigger retry
    pub fn with_no_retry_on_messages(mut self, messages: Vec<String>) -> Self {
        self.no_retry_on_messages = messages;
        self
    }

    /// Convert to BackoffConfig for use with backoff strategies
    pub fn to_backoff_config(&self) -> BackoffConfig {
        BackoffConfig {
            initial_delay: self.initial_delay,
            max_delay: self.max_delay,
            multiplier: self.backoff_multiplier,
            jitter: self.jitter,
            jitter_ratio: 0.2,
        }
    }

    /// Create an ExponentialBackoff from this config
    pub fn create_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff::with_config(self.to_backoff_config())
    }
}

/// Result of a retry operation
#[derive(Debug)]
pub enum RetryResult<T> {
    /// Operation succeeded
    Success(T),
    /// Operation failed after all retries exhausted
    Failed {
        /// The last error
        error: RecoverableError,
        /// Total attempts made
        attempts: u32,
        /// Total time spent
        elapsed: Duration,
    },
    /// Operation was cancelled
    Cancelled,
}

impl<T> RetryResult<T> {
    /// Check if the result is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Get the success value, if any
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Success(v) => Some(v),
            _ => None,
        }
    }

    /// Convert to Result, with error on failure or cancellation
    pub fn into_result(self) -> Result<T, RecoveryError> {
        match self {
            Self::Success(v) => Ok(v),
            Self::Failed { attempts, .. } => Err(RecoveryError::MaxRetriesExceeded { attempts }),
            Self::Cancelled => Err(RecoveryError::RecoveryTimeout {
                elapsed: Duration::ZERO,
            }),
        }
    }
}

/// Retry policy for operations
pub struct RetryPolicy {
    config: RetryConfig,
    backoff: Box<dyn BackoffStrategy>,
}

impl RetryPolicy {
    /// Create a new retry policy with default config and exponential backoff
    pub fn new() -> Self {
        let config = RetryConfig::default();
        let backoff = config.create_backoff();
        Self {
            config,
            backoff: Box::new(backoff),
        }
    }

    /// Create a new retry policy with custom config
    /// Uses the backoff settings from the config
    pub fn with_config(config: RetryConfig) -> Self {
        let backoff = config.create_backoff();
        Self {
            config,
            backoff: Box::new(backoff),
        }
    }

    /// Set custom backoff strategy
    pub fn with_backoff<B: BackoffStrategy + 'static>(mut self, backoff: B) -> Self {
        self.backoff = Box::new(backoff);
        self
    }

    /// Check if an error should be retried based on the config
    /// Note: attempt is 0-indexed, so max_attempts=3 allows attempts 0,1,2 (3 total)
    pub fn should_retry(&self, error: &SageError, attempt: u32) -> bool {
        // If we've already made max_attempts-1 retries, don't retry again
        // (attempt 0 is the first try, so attempt+1 is the total attempts so far)
        if attempt + 1 >= self.config.max_attempts {
            return false;
        }

        let error_str = error.to_string().to_lowercase();

        // Check no-retry list first
        for msg in &self.config.no_retry_on_messages {
            if error_str.contains(&msg.to_lowercase()) {
                return false;
            }
        }

        // Check always-retry list
        for msg in &self.config.retry_on_messages {
            if error_str.contains(&msg.to_lowercase()) {
                return true;
            }
        }

        // Use error classification
        match classify_error(error) {
            ErrorClass::Transient => true,
            ErrorClass::Permanent => false,
            ErrorClass::Unknown => self.config.retry_unknown,
        }
    }

    /// Execute an operation with retries
    pub async fn execute<T, F, Fut>(
        &mut self,
        mut operation: F,
        cancel_token: Option<CancellationToken>,
    ) -> RetryResult<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, SageError>>,
    {
        let start = std::time::Instant::now();
        let mut attempt = 0;
        let mut last_error: Option<RecoverableError> = None;

        loop {
            // Check cancellation
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return RetryResult::Cancelled;
                }
            }

            // Check total duration
            if start.elapsed() >= self.config.max_duration {
                return RetryResult::Failed {
                    error: last_error
                        .unwrap_or_else(|| RecoverableError::permanent("Max duration exceeded")),
                    attempts: attempt,
                    elapsed: start.elapsed(),
                };
            }

            // Execute operation
            match operation().await {
                Ok(result) => return RetryResult::Success(result),
                Err(error) => {
                    let recoverable = super::to_recoverable(&error);

                    if !self.should_retry(&error, attempt) {
                        return RetryResult::Failed {
                            error: recoverable,
                            attempts: attempt + 1,
                            elapsed: start.elapsed(),
                        };
                    }

                    // Get delay - use retry_after from error if available
                    let delay = recoverable
                        .retry_after
                        .unwrap_or_else(|| self.backoff.delay_for_attempt(attempt));

                    last_error = Some(recoverable);
                    attempt += 1;

                    // Wait before retry
                    if let Some(ref token) = cancel_token {
                        tokio::select! {
                            _ = token.cancelled() => {
                                return RetryResult::Cancelled;
                            }
                            _ = sleep(delay) => {}
                        }
                    } else {
                        sleep(delay).await;
                    }
                }
            }
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can be retried
pub trait Retryable {
    /// Get the retry classification for this error
    fn retry_class(&self) -> ErrorClass;

    /// Get suggested delay before retry
    fn retry_after(&self) -> Option<Duration> {
        None
    }
}

impl Retryable for SageError {
    fn retry_class(&self) -> ErrorClass {
        classify_error(self)
    }

    fn retry_after(&self) -> Option<Duration> {
        super::extract_retry_after(self)
    }
}

/// Convenience function to retry an async operation
pub async fn retry<T, F, Fut>(
    max_attempts: u32,
    operation: F,
    cancel_token: Option<CancellationToken>,
) -> RetryResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, SageError>>,
{
    let config = RetryConfig::limited(max_attempts);
    let mut policy = RetryPolicy::with_config(config);
    policy.execute(operation, cancel_token).await
}

/// Retry with custom config
pub async fn retry_with_config<T, F, Fut>(
    config: RetryConfig,
    operation: F,
    cancel_token: Option<CancellationToken>,
) -> RetryResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, SageError>>,
{
    let mut policy = RetryPolicy::with_config(config);
    policy.execute(operation, cancel_token).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_retry_success_immediately() {
        let result: RetryResult<i32> = retry(3, || async { Ok(42) }, None).await;

        assert!(result.is_success());
        assert_eq!(result.ok(), Some(42));
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: RetryResult<i32> = retry(
            5,
            || {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(SageError::http("timeout"))
                    } else {
                        Ok(42)
                    }
                }
            },
            None,
        )
        .await;

        assert!(result.is_success());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: RetryResult<i32> = retry(
            3,
            || {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err(SageError::http("timeout"))
                }
            },
            None,
        )
        .await;

        assert!(!result.is_success());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_permanent_error_no_retry() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: RetryResult<i32> = retry(
            5,
            || {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err(SageError::config("invalid config"))
                }
            },
            None,
        )
        .await;

        assert!(!result.is_success());
        // Should only attempt once for permanent errors
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_cancellation() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Cancel immediately
        token.cancel();

        let result: RetryResult<i32> = retry(
            5,
            || async { Err(SageError::http("timeout")) },
            Some(token_clone),
        )
        .await;

        assert!(matches!(result, RetryResult::Cancelled));
    }

    #[test]
    fn test_should_retry_logic() {
        let policy = RetryPolicy::new();

        // Transient errors should retry
        assert!(policy.should_retry(&SageError::http("timeout"), 0));
        assert!(policy.should_retry(&SageError::llm("rate limit"), 0));

        // Permanent errors should not retry
        assert!(!policy.should_retry(&SageError::config("invalid"), 0));
        assert!(!policy.should_retry(&SageError::http("401 unauthorized"), 0));

        // Max attempts check
        assert!(!policy.should_retry(&SageError::http("timeout"), 10));
    }
}
