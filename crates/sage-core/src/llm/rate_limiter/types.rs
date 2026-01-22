//! Rate limiter configuration and state types
//!
//! This module re-exports the unified `RateLimitConfig` from the recovery module
//! and provides internal state types for the LLM rate limiter.

use std::time::Instant;

// Re-export the unified RateLimitConfig from recovery module
pub use crate::recovery::rate_limiter::RateLimitConfig;

/// Internal state for the token bucket rate limiter
#[derive(Debug)]
pub(super) struct RateLimiterState {
    /// Current number of tokens available
    pub tokens: f64,
    /// Last time tokens were refilled
    pub last_refill: Instant,
}

impl RateLimiterState {
    /// Create a new rate limiter state with the given initial tokens
    pub fn new(initial_tokens: f64) -> Self {
        Self {
            tokens: initial_tokens,
            last_refill: Instant::now(),
        }
    }
}
