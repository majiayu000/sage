//! Rate limiter for LLM API calls
//!
//! Implements a token bucket rate limiter to prevent hitting provider rate limits
//! and avoid service disruption or cost overrun.

mod bucket;
mod limiter;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use bucket::RateLimiter;
pub use types::RateLimitConfig;

// Re-export global registry functions
pub mod global {
    pub use super::limiter::{disable_rate_limit, get_rate_limiter, set_rate_limit};
}
