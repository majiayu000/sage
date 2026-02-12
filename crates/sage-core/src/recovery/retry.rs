//! Retry configuration for handling transient failures

use serde::{Deserialize, Serialize};
use std::time::Duration;

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
            no_retry_on_messages: vec!["constraint".to_string(), "syntax".to_string()],
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
}
