//! Token bucket implementation for rate limiting
//!
//! Implements the "leaky bucket as a meter" algorithm.

use super::types::{RateLimitConfig, RateLimiterState};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Token bucket rate limiter
///
/// Allows a configurable sustained rate with bursts up to the bucket capacity.
/// Uses the "leaky bucket as a meter" algorithm.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        let state = RateLimiterState::new(config.burst_size as f64);

        Self {
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
        self.config.enabled
    }

    /// Get the current configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Try to acquire a token, waiting if necessary
    ///
    /// Returns the wait duration if the caller had to wait.
    pub async fn acquire(&self) -> Option<Duration> {
        if !self.config.enabled {
            return None;
        }

        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            debug!(
                "Rate limiter: acquired token, {} remaining",
                state.tokens as u32
            );
            None
        } else {
            // Calculate wait time until a token is available
            let tokens_needed = 1.0 - state.tokens;
            let tokens_per_second = self.config.requests_per_minute as f64 / 60.0;
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
    /// Returns true if a token was acquired, false if rate limited.
    pub async fn try_acquire(&self) -> bool {
        if !self.config.enabled {
            return true;
        }

        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Check current token count without consuming
    pub async fn available_tokens(&self) -> u32 {
        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);
        state.tokens as u32
    }

    /// Refill tokens based on elapsed time
    fn refill_tokens(&self, state: &mut RateLimiterState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);
        let elapsed_seconds = elapsed.as_secs_f64();

        // Calculate tokens to add
        let tokens_per_second = self.config.requests_per_minute as f64 / 60.0;
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
        }
    }
}
