//! Rate limiting for API calls
//!
//! This module provides rate limiting to control API request rates
//! and avoid hitting provider rate limits.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// Rate limiter using token bucket algorithm
#[derive(Debug)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimiterConfig,
    /// Current tokens available
    tokens: Arc<Mutex<f64>>,
    /// Last refill time
    last_refill: Arc<Mutex<Instant>>,
    /// Semaphore for concurrency limiting
    concurrent_semaphore: Arc<Semaphore>,
}

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

impl RateLimiter {
    /// Create a new rate limiter with default configuration
    pub fn new() -> Self {
        Self::with_config(RateLimiterConfig::default())
    }

    /// Create a new rate limiter with custom configuration
    pub fn with_config(config: RateLimiterConfig) -> Self {
        let tokens = config.burst_size as f64;
        Self {
            concurrent_semaphore: Arc::new(Semaphore::new(config.max_concurrent as usize)),
            config,
            tokens: Arc::new(Mutex::new(tokens)),
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill(&self) {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;

        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let new_tokens = elapsed * self.config.requests_per_second;

        *tokens = (*tokens + new_tokens).min(self.config.burst_size as f64);
        *last_refill = now;
    }

    /// Try to acquire a token without waiting
    pub async fn try_acquire(&self) -> Option<RateLimitGuard> {
        // Check concurrency limit
        let permit = self.concurrent_semaphore.clone().try_acquire_owned().ok()?;

        // Check token bucket
        self.refill().await;
        let mut tokens = self.tokens.lock().await;

        if *tokens >= 1.0 {
            *tokens -= 1.0;
            Some(RateLimitGuard {
                _permit: Some(permit),
            })
        } else {
            // Return permit if we can't get a token
            drop(permit);
            None
        }
    }

    /// Acquire a token, waiting if necessary
    pub async fn acquire(&self) -> Result<RateLimitGuard, RateLimitError> {
        let start = Instant::now();

        // First acquire concurrency permit
        let permit = if self.config.blocking {
            match tokio::time::timeout(
                self.config.max_wait,
                self.concurrent_semaphore.clone().acquire_owned(),
            )
            .await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(_)) => return Err(RateLimitError::Closed),
                Err(_) => {
                    return Err(RateLimitError::Timeout {
                        waited: start.elapsed(),
                    });
                }
            }
        } else {
            match self.concurrent_semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    return Err(RateLimitError::ConcurrencyExceeded {
                        max: self.config.max_concurrent,
                    });
                }
            }
        };

        // Now acquire token bucket token
        loop {
            if start.elapsed() >= self.config.max_wait {
                return Err(RateLimitError::Timeout {
                    waited: start.elapsed(),
                });
            }

            self.refill().await;
            let mut tokens = self.tokens.lock().await;

            if *tokens >= 1.0 {
                *tokens -= 1.0;
                return Ok(RateLimitGuard {
                    _permit: Some(permit),
                });
            }

            if !self.config.blocking {
                return Err(RateLimitError::WouldBlock);
            }

            // Calculate wait time for next token
            let tokens_needed = 1.0 - *tokens;
            let wait_secs = tokens_needed / self.config.requests_per_second;
            let wait_duration = Duration::from_secs_f64(wait_secs).min(Duration::from_millis(100));

            drop(tokens);
            sleep(wait_duration).await;
        }
    }

    /// Get current available tokens
    pub async fn available_tokens(&self) -> f64 {
        self.refill().await;
        *self.tokens.lock().await
    }

    /// Get current concurrency usage
    pub fn concurrent_requests(&self) -> usize {
        self.config.max_concurrent as usize - self.concurrent_semaphore.available_permits()
    }

    /// Check if rate limited (would need to wait)
    pub async fn is_limited(&self) -> bool {
        self.available_tokens().await < 1.0
    }

    /// Get configuration
    pub fn config(&self) -> &RateLimiterConfig {
        &self.config
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard returned when a rate limit token is acquired
#[derive(Debug)]
pub struct RateLimitGuard {
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
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

/// Sliding window rate limiter for more accurate rate limiting
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    /// Window size
    window_size: Duration,
    /// Maximum requests per window
    max_requests: u32,
    /// Request timestamps
    timestamps: Arc<Mutex<VecDeque<Instant>>>,
    /// Maximum wait time
    max_wait: Duration,
}

impl SlidingWindowRateLimiter {
    /// Create a new sliding window rate limiter
    pub fn new(max_requests: u32, window_size: Duration) -> Self {
        Self {
            window_size,
            max_requests,
            timestamps: Arc::new(Mutex::new(VecDeque::new())),
            max_wait: Duration::from_secs(60),
        }
    }

    /// Create with requests per minute
    pub fn per_minute(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(60))
    }

    /// Create with requests per second
    pub fn per_second(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(1))
    }

    /// Set max wait time
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait;
        self
    }

    /// Clean up old timestamps
    async fn cleanup(&self) {
        let mut timestamps = self.timestamps.lock().await;
        let cutoff = Instant::now() - self.window_size;

        while let Some(front) = timestamps.front() {
            if *front < cutoff {
                timestamps.pop_front();
            } else {
                break;
            }
        }
    }

    /// Try to record a request without waiting
    pub async fn try_record(&self) -> bool {
        self.cleanup().await;

        let mut timestamps = self.timestamps.lock().await;
        if timestamps.len() < self.max_requests as usize {
            timestamps.push_back(Instant::now());
            true
        } else {
            false
        }
    }

    /// Record a request, waiting if necessary
    pub async fn record(&self) -> Result<(), RateLimitError> {
        let start = Instant::now();

        loop {
            if start.elapsed() >= self.max_wait {
                return Err(RateLimitError::Timeout {
                    waited: start.elapsed(),
                });
            }

            self.cleanup().await;

            let mut timestamps = self.timestamps.lock().await;
            if timestamps.len() < self.max_requests as usize {
                timestamps.push_back(Instant::now());
                return Ok(());
            }

            // Calculate wait time until oldest request expires
            if let Some(oldest) = timestamps.front() {
                let age = Instant::now().duration_since(*oldest);
                if age < self.window_size {
                    let wait = self.window_size - age;
                    drop(timestamps);
                    sleep(wait.min(Duration::from_millis(100))).await;
                }
            }
        }
    }

    /// Get current request count in window
    pub async fn current_count(&self) -> usize {
        self.cleanup().await;
        self.timestamps.lock().await.len()
    }

    /// Check if rate limited
    pub async fn is_limited(&self) -> bool {
        self.cleanup().await;
        self.timestamps.lock().await.len() >= self.max_requests as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(100.0)
                .with_burst_size(5),
        );

        // Should be able to acquire burst_size tokens immediately
        for _ in 0..5 {
            assert!(limiter.try_acquire().await.is_some());
        }

        // Should be rate limited now
        assert!(limiter.try_acquire().await.is_none());
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(100.0)
                .with_burst_size(2),
        );

        // Use all tokens
        limiter.try_acquire().await;
        limiter.try_acquire().await;

        // Should be limited
        assert!(limiter.try_acquire().await.is_none());

        // Wait for refill
        sleep(Duration::from_millis(20)).await;

        // Should have tokens again
        assert!(limiter.try_acquire().await.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocking() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(1000.0)
                .with_burst_size(1),
        );

        // Use the token
        limiter.try_acquire().await;

        // Blocking acquire should eventually succeed
        let start = Instant::now();
        let result = limiter.acquire().await;
        assert!(result.is_ok());
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_rate_limiter_non_blocking() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(1.0)
                .with_burst_size(1)
                .non_blocking(),
        );

        // Use the token
        limiter.try_acquire().await;

        // Non-blocking acquire should fail immediately
        let result = limiter.acquire().await;
        assert!(matches!(result, Err(RateLimitError::WouldBlock)));
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrency() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(1000.0)
                .with_burst_size(100)
                .with_max_concurrent(2)
                .non_blocking(),
        );

        // Acquire two permits
        let g1 = limiter.acquire().await.unwrap();
        let g2 = limiter.acquire().await.unwrap();

        // Third should fail due to concurrency
        let result = limiter.acquire().await;
        assert!(matches!(
            result,
            Err(RateLimitError::ConcurrencyExceeded { .. })
        ));

        // Drop one permit
        drop(g1);

        // Should succeed now
        let _g3 = limiter.acquire().await.unwrap();

        drop(g2);
    }

    #[tokio::test]
    async fn test_rate_limiter_available_tokens() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(10.0)
                .with_burst_size(10),
        );

        let initial = limiter.available_tokens().await;
        assert!((initial - 10.0).abs() < 0.01);

        limiter.try_acquire().await;
        let after_one = limiter.available_tokens().await;
        assert!(after_one < initial);
    }

    #[tokio::test]
    async fn test_sliding_window_basic() {
        let limiter = SlidingWindowRateLimiter::new(3, Duration::from_millis(100));

        // Should allow 3 requests
        assert!(limiter.try_record().await);
        assert!(limiter.try_record().await);
        assert!(limiter.try_record().await);

        // Fourth should fail
        assert!(!limiter.try_record().await);

        // Wait for window to pass
        sleep(Duration::from_millis(110)).await;

        // Should allow again
        assert!(limiter.try_record().await);
    }

    #[tokio::test]
    async fn test_sliding_window_per_second() {
        let limiter = SlidingWindowRateLimiter::per_second(2);

        assert!(limiter.try_record().await);
        assert!(limiter.try_record().await);
        assert!(!limiter.try_record().await);

        assert!(limiter.is_limited().await);
    }

    #[tokio::test]
    async fn test_sliding_window_blocking() {
        let limiter = SlidingWindowRateLimiter::new(1, Duration::from_millis(50))
            .with_max_wait(Duration::from_secs(1));

        // Use the slot
        limiter.record().await.unwrap();

        // Blocking record should eventually succeed
        let start = Instant::now();
        limiter.record().await.unwrap();
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_sliding_window_current_count() {
        let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(10));

        assert_eq!(limiter.current_count().await, 0);

        limiter.try_record().await;
        limiter.try_record().await;

        assert_eq!(limiter.current_count().await, 2);
    }

    #[test]
    fn test_rate_limit_error_display() {
        let timeout = RateLimitError::Timeout {
            waited: Duration::from_secs(30),
        };
        assert!(timeout.to_string().contains("30"));

        let concurrency = RateLimitError::ConcurrencyExceeded { max: 5 };
        assert!(concurrency.to_string().contains("5"));
    }

    #[test]
    fn test_config_presets() {
        let anthropic = RateLimiterConfig::for_anthropic();
        assert!(anthropic.requests_per_second >= 50.0);

        let openai = RateLimiterConfig::for_openai();
        assert!(openai.requests_per_second >= 60.0);

        let conservative = RateLimiterConfig::conservative();
        assert!(conservative.requests_per_second <= 2.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent_requests() {
        let limiter = RateLimiter::with_config(RateLimiterConfig::default().with_max_concurrent(3));

        assert_eq!(limiter.concurrent_requests(), 0);

        let _g1 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.concurrent_requests(), 1);

        let _g2 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.concurrent_requests(), 2);
    }
}
