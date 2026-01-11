//! Rate limiter configuration and state types

use std::time::Instant;

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum burst size (allows short bursts above the sustained rate)
    pub burst_size: u32,
    /// Maximum concurrent requests (0 = unlimited)
    pub max_concurrent: u32,
    /// Whether rate limiting is enabled
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            // Default: 60 requests per minute (1 per second average)
            requests_per_minute: 60,
            // Allow bursts of up to 10 requests
            burst_size: 10,
            // Default: 5 concurrent requests
            max_concurrent: 5,
            enabled: true,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        Self {
            requests_per_minute,
            burst_size,
            max_concurrent: 5,
            enabled: true,
        }
    }

    /// Create a rate limit configuration with concurrent limit
    pub fn with_concurrent(requests_per_minute: u32, burst_size: u32, max_concurrent: u32) -> Self {
        Self {
            requests_per_minute,
            burst_size,
            max_concurrent,
            enabled: true,
        }
    }

    /// Create a disabled rate limiter
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Get configuration for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        match provider.to_lowercase().as_str() {
            // OpenAI: Varies by tier, use conservative defaults
            "openai" => Self::with_concurrent(60, 20, 8),
            // Anthropic: 60 RPM for Claude models
            "anthropic" => Self::with_concurrent(60, 10, 5),
            // Google: 60 RPM for Gemini
            "google" => Self::with_concurrent(60, 15, 6),
            // Azure: Depends on deployment, use conservative
            "azure" => Self::with_concurrent(60, 20, 8),
            // Doubao: Similar to OpenAI
            "doubao" => Self::with_concurrent(60, 20, 8),
            // OpenRouter: Aggregates multiple providers
            "openrouter" => Self::with_concurrent(60, 20, 10),
            // Ollama: Local, can be more generous
            "ollama" => Self::with_concurrent(120, 30, 20),
            // GLM: Conservative defaults
            "glm" => Self::with_concurrent(60, 15, 6),
            // Default for unknown providers
            _ => Self::default(),
        }
    }
}

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
