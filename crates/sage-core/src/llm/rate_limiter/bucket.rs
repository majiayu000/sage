//! LLM-specific rate limiter wrapper
//!
//! This module provides an LLM-friendly API on top of the shared token bucket
//! rate limiter from `crate::recovery::rate_limiter`. Instead of duplicating
//! the token bucket algorithm, it delegates to the recovery `RateLimiter` and
//! adapts the return types for LLM client usage.

use super::types::RateLimitConfig;
use crate::recovery::rate_limiter::RateLimiter as CoreRateLimiter;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Rate limiter for LLM API calls
///
/// Wraps the shared `recovery::rate_limiter::RateLimiter` (token bucket + semaphore)
/// and provides an LLM-friendly API that returns wait durations and booleans
/// instead of guards and results.
///
/// Cloning this struct shares the underlying state (token bucket and semaphore),
/// so multiple clones coordinate rate limiting together.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// The underlying shared rate limiter implementation, wrapped in Arc
    /// so that clones share the same token bucket and concurrency state.
    inner: Arc<CoreRateLimiter>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            inner: Arc::new(CoreRateLimiter::with_config(config)),
        }
    }

    /// Create a rate limiter for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        Self::new(RateLimitConfig::for_provider(provider))
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.inner.config().enabled && !self.inner.config().is_disabled()
    }

    /// Get the current configuration
    pub fn config(&self) -> &RateLimitConfig {
        self.inner.config()
    }

    /// Try to acquire a token, waiting if necessary
    ///
    /// Returns the wait duration if the caller had to wait.
    /// Returns `None` if a token was immediately available or rate limiting is disabled.
    pub async fn acquire(&self) -> Option<Duration> {
        if !self.inner.config().enabled {
            return None;
        }

        let start = Instant::now();

        // Delegate to the core rate limiter's blocking acquire.
        // The core limiter returns Result<RateLimitGuard, RateLimitError>.
        // We adapt this to Option<Duration> for the LLM API.
        match self.inner.acquire().await {
            Ok(_guard) => {
                // Guard is dropped immediately -- the LLM rate limiter does not
                // hold the concurrency permit for the duration of the request.
                // This matches the original behavior.
                let elapsed = start.elapsed();
                if elapsed > Duration::from_millis(5) {
                    warn!(
                        "Rate limiter: waited {:.2}s for token",
                        elapsed.as_secs_f64()
                    );
                    Some(elapsed)
                } else {
                    debug!(
                        "Rate limiter: acquired token, {} concurrent",
                        self.concurrent_requests()
                    );
                    None
                }
            }
            Err(e) => {
                warn!("Rate limiter: acquire failed: {}", e);
                None
            }
        }
    }

    /// Try to acquire a token without waiting
    ///
    /// Returns true if a token was acquired, false if rate limited or at max concurrency.
    pub async fn try_acquire(&self) -> bool {
        if !self.inner.config().enabled {
            return true;
        }

        // Delegate to the core rate limiter's non-blocking try_acquire.
        // The guard is dropped immediately, releasing the concurrency permit.
        self.inner.try_acquire().await.is_some()
    }

    /// Check current token count without consuming
    pub async fn available_tokens(&self) -> u32 {
        let tokens = self.inner.available_tokens().await;
        // Safely convert f64 to u32: clamp to valid range, default to 0 for invalid values
        if tokens.is_finite() && tokens >= 0.0 {
            let clamped = tokens.min(u32::MAX as f64);
            clamped as u32
        } else {
            0
        }
    }

    /// Get current number of concurrent requests
    ///
    /// Returns the number of requests currently in progress.
    pub fn concurrent_requests(&self) -> usize {
        self.inner.concurrent_requests()
    }

    /// Get the maximum allowed concurrent requests
    pub fn max_concurrent(&self) -> u32 {
        self.inner.config().max_concurrent
    }
}
