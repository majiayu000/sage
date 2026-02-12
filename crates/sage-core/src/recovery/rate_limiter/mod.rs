//! Rate limiting for API calls
//!
//! This module provides rate limiting to control API request rates
//! and avoid hitting provider rate limits.

mod limiter;
mod types;

#[cfg(test)]
mod tests;

pub use limiter::RateLimiter;
pub use types::{RateLimitConfig, RateLimitError};
