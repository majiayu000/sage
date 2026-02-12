//! Type definitions for rate limiting
//!
//! This module provides a unified `RateLimitConfig` that supports multiple use cases:
//! - General API rate limiting (requests per second/minute)
//! - LLM-specific rate limiting (tokens per minute)
//! - Concurrent request limiting
//! - Blocking vs non-blocking modes

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Unified rate limit configuration
///
/// This configuration supports multiple rate limiting scenarios:
/// - API rate limiting with requests per second or minute
/// - LLM token-based rate limiting
/// - Concurrent request limiting
/// - Blocking and non-blocking modes
///
/// # Examples
///
/// ```ignore
/// use sage_core::recovery::rate_limiter::RateLimitConfig;
///
/// // For general API use
/// let api_config = RateLimitConfig::for_api(100, 20);
///
/// // For LLM providers
/// let llm_config = RateLimitConfig::for_provider("anthropic");
///
/// // Custom configuration
/// let custom = RateLimitConfig::default()
///     .with_requests_per_minute(60)
///     .with_burst_size(10)
///     .with_max_concurrent(5);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute (primary rate limit)
    /// If None, rate limiting by request count is disabled
    #[serde(default)]
    pub requests_per_minute: Option<u32>,

    /// Maximum tokens per minute (LLM-specific)
    /// If None, token-based rate limiting is disabled
    #[serde(default)]
    pub tokens_per_minute: Option<u32>,

    /// Maximum burst size (token bucket capacity)
    /// Allows short bursts above the sustained rate
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,

    /// Maximum concurrent requests (0 = unlimited)
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,

    /// Whether rate limiting is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Whether to block waiting for tokens or reject immediately
    #[serde(default = "default_blocking")]
    pub blocking: bool,

    /// Maximum time to wait for a token (only used when blocking=true)
    #[serde(default = "default_max_wait", with = "duration_serde")]
    pub max_wait: Duration,
}

fn default_burst_size() -> u32 {
    10
}
fn default_max_concurrent() -> u32 {
    5
}
fn default_enabled() -> bool {
    true
}
fn default_blocking() -> bool {
    true
}
fn default_max_wait() -> Duration {
    Duration::from_secs(30)
}

/// Serde support for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: Some(60),
            tokens_per_minute: None,
            burst_size: 10,
            max_concurrent: 5,
            enabled: true,
            blocking: true,
            max_wait: Duration::from_secs(30),
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration with requests per minute
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        Self {
            requests_per_minute: Some(requests_per_minute),
            burst_size,
            ..Default::default()
        }
    }

    /// Create a rate limit configuration with concurrent limit
    pub fn with_concurrent(requests_per_minute: u32, burst_size: u32, max_concurrent: u32) -> Self {
        Self {
            requests_per_minute: Some(requests_per_minute),
            burst_size,
            max_concurrent,
            ..Default::default()
        }
    }

    /// Create a disabled rate limiter
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    // ========== Factory methods for different use cases ==========

    /// Create config for general API rate limiting
    pub fn for_api(requests_per_minute: u32, burst_size: u32) -> Self {
        Self {
            requests_per_minute: Some(requests_per_minute),
            tokens_per_minute: None,
            burst_size,
            max_concurrent: 10,
            enabled: true,
            blocking: true,
            max_wait: Duration::from_secs(30),
        }
    }

    /// Create config for LLM provider with typical limits
    pub fn for_llm(requests_per_minute: u32, tokens_per_minute: u32, max_concurrent: u32) -> Self {
        Self {
            requests_per_minute: Some(requests_per_minute),
            tokens_per_minute: Some(tokens_per_minute),
            burst_size: requests_per_minute / 6, // ~10 seconds worth
            max_concurrent,
            enabled: true,
            blocking: true,
            max_wait: Duration::from_secs(60),
        }
    }

    /// Get configuration for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        match provider.to_lowercase().as_str() {
            // OpenAI: Varies by tier, use conservative defaults
            "openai" => Self::for_llm(60, 100_000, 10),
            // Anthropic: 60 RPM for Claude models
            "anthropic" => Self::for_llm(50, 80_000, 5),
            // Google: 60 RPM for Gemini
            "google" => Self::for_llm(60, 120_000, 10),
            // Azure: Depends on deployment, use conservative
            "azure" => Self::for_llm(60, 100_000, 10),
            // Doubao: Similar to OpenAI
            "doubao" => Self::for_llm(60, 100_000, 10),
            // OpenRouter: Aggregates multiple providers
            "openrouter" => Self::for_llm(60, 100_000, 10),
            // Ollama: Local, can be more generous (no token limit)
            "ollama" => Self {
                requests_per_minute: Some(120),
                tokens_per_minute: None,
                burst_size: 30,
                max_concurrent: 20,
                enabled: true,
                blocking: true,
                max_wait: Duration::from_secs(120),
            },
            // GLM: Conservative defaults
            "glm" => Self::for_llm(60, 100_000, 6),
            // Default for unknown providers
            _ => Self::default(),
        }
    }

    /// Create config for Claude API (typical limits)
    pub fn for_anthropic() -> Self {
        Self::for_provider("anthropic")
    }

    /// Create config for OpenAI API
    pub fn for_openai() -> Self {
        Self::for_provider("openai")
    }

    /// Create config for conservative rate limiting
    pub fn conservative() -> Self {
        Self {
            requests_per_minute: Some(60),
            tokens_per_minute: Some(50_000),
            burst_size: 5,
            max_concurrent: 2,
            enabled: true,
            blocking: true,
            max_wait: Duration::from_secs(120),
        }
    }

    // ========== Builder methods ==========

    /// Set requests per minute
    pub fn with_requests_per_minute(mut self, rpm: u32) -> Self {
        self.requests_per_minute = Some(rpm);
        self
    }

    /// Set tokens per minute (LLM-specific)
    pub fn with_tokens_per_minute(mut self, tpm: u32) -> Self {
        self.tokens_per_minute = Some(tpm);
        self
    }

    /// Set burst size
    pub fn with_burst_size(mut self, size: u32) -> Self {
        self.burst_size = size;
        self
    }

    /// Set max concurrent requests
    pub fn with_max_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Enable or disable rate limiting
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set to non-blocking mode
    pub fn non_blocking(mut self) -> Self {
        self.blocking = false;
        self
    }

    /// Set blocking mode
    pub fn with_blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }

    /// Set maximum wait time
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait;
        self
    }

    // ========== Conversion helpers ==========

    /// Get requests per second (converts from requests_per_minute)
    pub fn requests_per_second(&self) -> f64 {
        self.requests_per_minute
            .map(|rpm| rpm as f64 / 60.0)
            .unwrap_or(f64::MAX)
    }

    /// Set requests per second (converts to requests_per_minute)
    pub fn with_rps(mut self, rps: f64) -> Self {
        if rps.is_finite() && rps > 0.0 {
            let rpm = rps * 60.0;
            let bounded = rpm.min(u32::MAX as f64);
            self.requests_per_minute = Some(bounded as u32);
        } else {
            self.requests_per_minute = Some(0);
        }
        self
    }

    /// Check if rate limiting is effectively disabled
    pub fn is_disabled(&self) -> bool {
        !self.enabled || (self.requests_per_minute.is_none() && self.tokens_per_minute.is_none())
    }
}

/// Guard returned when a rate limit token is acquired
#[derive(Debug)]
pub struct RateLimitGuard {
    pub(crate) _permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

/// Rate limit errors
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitError {
    /// Timeout waiting for token
    Timeout { waited: Duration },
    /// Would block in non-blocking mode
    WouldBlock,
    /// Concurrency limit exceeded
    ConcurrencyExceeded { max: u32 },
    /// Rate limiter is closed
    Closed,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { waited } => write!(f, "Rate limit timeout after {:?}", waited),
            Self::WouldBlock => write!(f, "Rate limit would block"),
            Self::ConcurrencyExceeded { max } => {
                write!(f, "Concurrency limit exceeded (max {})", max)
            }
            Self::Closed => write!(f, "Rate limiter closed"),
        }
    }
}

impl std::error::Error for RateLimitError {}
