//! Rate limiter configuration types for LLM rate limiting
//!
//! This module re-exports the unified `RateLimitConfig` from the recovery module.
//! The LLM rate limiter (`bucket.rs`) delegates to the shared token bucket
//! implementation in `recovery::rate_limiter`, so no internal state types are needed here.

// Re-export the unified RateLimitConfig from recovery module
pub use crate::recovery::rate_limiter::RateLimitConfig;
