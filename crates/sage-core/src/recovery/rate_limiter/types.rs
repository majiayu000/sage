//! Type definitions for rate limiting

use std::time::Duration;

/// Configuration for rate limiter
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per second
    pub requests_per_second: f64,
    /// Maximum burst size (token bucket capacity)
    pub burst_size: u32,
    /// Maximum concurrent requests
    pub max_concurrent: u32,
    /// Whether to block waiting for tokens or reject immediately
    pub blocking: bool,
    /// Maximum time to wait for a token
    pub max_wait: Duration,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            burst_size: 20,
            max_concurrent: 5,
            blocking: true,
            max_wait: Duration::from_secs(30),
        }
    }
}

impl RateLimiterConfig {
    /// Create config for Claude API (typical limits)
    pub fn for_anthropic() -> Self {
        Self {
            requests_per_second: 50.0,
            burst_size: 100,
            max_concurrent: 10,
            blocking: true,
            max_wait: Duration::from_secs(60),
        }
    }

    /// Create config for OpenAI API
    pub fn for_openai() -> Self {
        Self {
            requests_per_second: 60.0,
            burst_size: 60,
            max_concurrent: 10,
            blocking: true,
            max_wait: Duration::from_secs(60),
        }
    }

    /// Create config for conservative rate limiting
    pub fn conservative() -> Self {
        Self {
            requests_per_second: 1.0,
            burst_size: 5,
            max_concurrent: 2,
            blocking: true,
            max_wait: Duration::from_secs(120),
        }
    }

    /// Set requests per second
    pub fn with_rps(mut self, rps: f64) -> Self {
        self.requests_per_second = rps;
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

    /// Set to non-blocking mode
    pub fn non_blocking(mut self) -> Self {
        self.blocking = false;
        self
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
