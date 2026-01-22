//! Resilience configuration (retry and rate limiting)

use serde::{Deserialize, Serialize};

// Re-export the unified RateLimitConfig from recovery module
pub use crate::recovery::rate_limiter::RateLimitConfig;

/// Resilience configuration for retry and rate limiting
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResilienceConfig {
    /// Maximum number of retries for failed requests
    pub max_retries: Option<u32>,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
}

impl ResilienceConfig {
    /// Create a new resilience config with default settings
    pub fn new() -> Self {
        Self {
            max_retries: Some(3),
            rate_limit: None,
        }
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set rate limiting configuration
    pub fn with_rate_limit(mut self, rate_limit: RateLimitConfig) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }
}
