//! Retry policies and utilities for handling transient failures
//!
//! Provides configurable retry behavior with backoff strategies.

use super::backoff::{BackoffStrategy, ExponentialBackoff};
use super::{ErrorClass, RecoverableError, RecoveryError, classify_error};
use crate::error::SageError;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Maximum total time to spend retrying
    pub max_duration: Duration,
    /// Whether to retry on unknown errors
    pub retry_unknown: bool,
    /// Specific error messages to always retry
    pub retry_on_messages: Vec<String>,
    /// Specific error messages to never retry
    pub no_retry_on_messages: Vec<String>,
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
        }
    }
}

impl RetryConfig {
    /// Create a config for aggressive retrying
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            max_duration: Duration::from_secs(600),
            retry_unknown: true,
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
        Self {
            config: RetryConfig::default(),
            backoff: Box::new(ExponentialBackoff::new()),
        }
    }

    /// Create a new retry policy with custom config
    pub fn with_config(config: RetryConfig) -> Self {
        Self {
            config,
            backoff: Box::new(ExponentialBackoff::new()),
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
                        Err(SageError::Http("timeout".into()))
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
                    Err(SageError::Http("timeout".into()))
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
                    Err(SageError::Config("invalid config".into()))
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
            || async { Err(SageError::Http("timeout".into())) },
            Some(token_clone),
        )
        .await;

        assert!(matches!(result, RetryResult::Cancelled));
    }

    #[test]
    fn test_should_retry_logic() {
        let policy = RetryPolicy::new();

        // Transient errors should retry
        assert!(policy.should_retry(&SageError::Http("timeout".into()), 0));
        assert!(policy.should_retry(&SageError::Llm("rate limit".into()), 0));

        // Permanent errors should not retry
        assert!(!policy.should_retry(&SageError::Config("invalid".into()), 0));
        assert!(!policy.should_retry(&SageError::Http("401 unauthorized".into()), 0));

        // Max attempts check
        assert!(!policy.should_retry(&SageError::Http("timeout".into()), 10));
    }
}
