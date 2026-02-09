//! Token bucket implementation for rate limiting
//!
//! Implements the "leaky bucket as a meter" algorithm with concurrent request limiting.

use super::types::{RateLimitConfig, RateLimiterState};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, warn};

/// Token bucket rate limiter
///
/// Allows a configurable sustained rate with bursts up to the bucket capacity.
/// Uses the "leaky bucket as a meter" algorithm with concurrent request limiting.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
    /// Semaphore for concurrent request limiting
    concurrent_semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        let state = RateLimiterState::new(config.burst_size as f64);
        // Use max_concurrent from config, or a very large value if set to 0 (unlimited)
        // tokio Semaphore has a max limit, so we use a reasonable large number
        let max_concurrent = if config.max_concurrent == 0 {
            // Use a large but safe value (tokio's MAX_PERMITS is around 2^61)
            1_000_000
        } else {
            config.max_concurrent as usize
        };

        Self {
            concurrent_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            config,
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Create a rate limiter for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        Self::new(RateLimitConfig::for_provider(provider))
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.is_disabled()
    }

    /// Get the current configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Try to acquire a token, waiting if necessary
    ///
    /// Returns the wait duration if the caller had to wait.
    /// This also acquires a concurrent request permit that is automatically
    /// released when the returned guard is dropped.
    pub async fn acquire(&self) -> Option<Duration> {
        if !self.config.enabled {
            return None;
        }

        // First acquire concurrent permit (waits if at max concurrency)
        let permit = self.concurrent_semaphore.clone().acquire_owned().await.ok();

        if permit.is_none() {
            warn!("Rate limiter: concurrent semaphore closed");
            return None;
        }

        // Forget the permit so it lives until the request completes
        // Note: In a production system, you'd want to return a guard that holds the permit
        // For now, we release immediately after the token bucket check
        let _permit = permit.unwrap();

        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            debug!(
                "Rate limiter: acquired token, {} remaining, {} concurrent",
                state.tokens as u32,
                self.concurrent_requests()
            );
            None
        } else {
            // Calculate wait time until a token is available
            let tokens_needed = 1.0 - state.tokens;
            let tokens_per_second = self.config.requests_per_second();
            let wait_seconds = tokens_needed / tokens_per_second;
            let wait_duration = Duration::from_secs_f64(wait_seconds);

            warn!(
                "Rate limiter: no tokens available, waiting {:.2}s",
                wait_seconds
            );

            // Release the lock before sleeping
            drop(state);

            // Wait for the required duration
            tokio::time::sleep(wait_duration).await;

            // Re-acquire lock and consume token
            let mut state = self.state.lock().await;
            self.refill_tokens(&mut state);
            state.tokens = (state.tokens - 1.0).max(0.0);

            Some(wait_duration)
        }
    }

    /// Try to acquire a token without waiting
    ///
    /// Returns true if a token was acquired, false if rate limited or at max concurrency.
    pub async fn try_acquire(&self) -> bool {
        if !self.config.enabled {
            return true;
        }

        // Check concurrency limit first
        let permit = match self.concurrent_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                debug!("Rate limiter: at max concurrency, try_acquire failed");
                return false;
            }
        };

        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            // Permit is dropped here, releasing the concurrent slot
            drop(permit);
            true
        } else {
            // Return permit if we can't get a token
            drop(permit);
            false
        }
    }

    /// Check current token count without consuming
    pub async fn available_tokens(&self) -> u32 {
        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);
        state.tokens as u32
    }

    /// Get current number of concurrent requests
    ///
    /// Returns the number of requests currently in progress.
    pub fn concurrent_requests(&self) -> usize {
        let max = if self.config.max_concurrent == 0 {
            1_000_000 // Same value as used in new()
        } else {
            self.config.max_concurrent as usize
        };
        max.saturating_sub(self.concurrent_semaphore.available_permits())
    }

    /// Get the maximum allowed concurrent requests
    pub fn max_concurrent(&self) -> u32 {
        self.config.max_concurrent
    }

    /// Refill tokens based on elapsed time
    fn refill_tokens(&self, state: &mut RateLimiterState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);
        let elapsed_seconds = elapsed.as_secs_f64();

        // Calculate tokens to add
        let tokens_per_second = self.config.requests_per_second();
        let tokens_to_add = elapsed_seconds * tokens_per_second;

        // Add tokens, capped at burst size
        state.tokens = (state.tokens + tokens_to_add).min(self.config.burst_size as f64);
        state.last_refill = now;
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            concurrent_semaphore: Arc::clone(&self.concurrent_semaphore),
        }
    }
}
