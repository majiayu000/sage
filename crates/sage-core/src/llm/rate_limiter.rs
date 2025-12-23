//! Rate limiter for LLM API calls
//!
//! Implements a token bucket rate limiter to prevent hitting provider rate limits
//! and avoid service disruption or cost overrun.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum burst size (allows short bursts above the sustained rate)
    pub burst_size: u32,
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
            "openai" => Self::new(60, 20),
            // Anthropic: 60 RPM for Claude models
            "anthropic" => Self::new(60, 10),
            // Google: 60 RPM for Gemini
            "google" => Self::new(60, 15),
            // Azure: Depends on deployment, use conservative
            "azure" => Self::new(60, 20),
            // Doubao: Similar to OpenAI
            "doubao" => Self::new(60, 20),
            // OpenRouter: Aggregates multiple providers
            "openrouter" => Self::new(60, 20),
            // Ollama: Local, can be more generous
            "ollama" => Self::new(120, 30),
            // GLM: Conservative defaults
            "glm" => Self::new(60, 15),
            // Default for unknown providers
            _ => Self::default(),
        }
    }
}

/// Token bucket rate limiter
///
/// Allows a configurable sustained rate with bursts up to the bucket capacity.
/// Uses the "leaky bucket as a meter" algorithm.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

#[derive(Debug)]
struct RateLimiterState {
    /// Current number of tokens available
    tokens: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        let state = RateLimiterState {
            tokens: config.burst_size as f64,
            last_refill: Instant::now(),
        };

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

/// Global rate limiter registry for per-provider rate limiting
pub mod global {
    use super::*;
    use std::collections::HashMap;
    use std::sync::OnceLock;
    use tokio::sync::RwLock;

    static RATE_LIMITERS: OnceLock<RwLock<HashMap<String, RateLimiter>>> = OnceLock::new();

    fn get_registry() -> &'static RwLock<HashMap<String, RateLimiter>> {
        RATE_LIMITERS.get_or_init(|| RwLock::new(HashMap::new()))
    }

    /// Get or create a rate limiter for the given provider
    pub async fn get_rate_limiter(provider: &str) -> RateLimiter {
        let provider_key = provider.to_lowercase();

        // Try to read first
        {
            let registry = get_registry().read().await;
            if let Some(limiter) = registry.get(&provider_key) {
                return limiter.clone();
            }
        }

        // Create new limiter
        let mut registry = get_registry().write().await;
        // Double-check after acquiring write lock
        if let Some(limiter) = registry.get(&provider_key) {
            return limiter.clone();
        }

        let limiter = RateLimiter::for_provider(&provider_key);
        registry.insert(provider_key, limiter.clone());
        limiter
    }

    /// Update rate limit configuration for a provider
    pub async fn set_rate_limit(provider: &str, config: RateLimitConfig) {
        let provider_key = provider.to_lowercase();
        let mut registry = get_registry().write().await;
        registry.insert(provider_key, RateLimiter::new(config));
    }

    /// Disable rate limiting for a provider
    pub async fn disable_rate_limit(provider: &str) {
        let provider_key = provider.to_lowercase();
        let mut registry = get_registry().write().await;
        registry.insert(provider_key, RateLimiter::new(RateLimitConfig::disabled()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_burst() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 5,
            enabled: true,
        });

        // Should be able to acquire burst_size tokens immediately
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }

        // 6th request should fail (no waiting)
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let limiter = RateLimiter::new(RateLimitConfig::disabled());

        // Should always succeed when disabled
        for _ in 0..100 {
            assert!(limiter.try_acquire().await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 600, // 10 per second for faster test
            burst_size: 2,
            enabled: true,
        });

        // Exhaust tokens
        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
        assert!(!limiter.try_acquire().await);

        // Wait for refill (100ms should add 1 token at 10/sec)
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should have 1 token now
        assert!(limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_available_tokens() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 5,
            enabled: true,
        });

        assert_eq!(limiter.available_tokens().await, 5);

        limiter.try_acquire().await;
        assert_eq!(limiter.available_tokens().await, 4);
    }

    #[tokio::test]
    async fn test_provider_configs() {
        // Test that provider-specific configs are reasonable
        let openai = RateLimitConfig::for_provider("openai");
        assert!(openai.requests_per_minute >= 60);

        let anthropic = RateLimitConfig::for_provider("anthropic");
        assert!(anthropic.requests_per_minute >= 60);

        let ollama = RateLimitConfig::for_provider("ollama");
        assert!(ollama.requests_per_minute >= 60);
    }

    #[tokio::test]
    async fn test_acquire_waits() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 600, // 10 per second for faster test
            burst_size: 1,
            enabled: true,
        });

        // First should not wait
        let wait1 = limiter.acquire().await;
        assert!(wait1.is_none());

        // Second should wait (bucket empty)
        let start = Instant::now();
        let wait2 = limiter.acquire().await;
        let elapsed = start.elapsed();

        assert!(wait2.is_some());
        assert!(elapsed >= Duration::from_millis(90)); // ~100ms expected
    }

    #[tokio::test]
    async fn test_global_registry() {
        // Get limiter for a provider
        let limiter1 = global::get_rate_limiter("test_provider").await;
        let limiter2 = global::get_rate_limiter("test_provider").await;

        // Both should share the same state
        limiter1.try_acquire().await;
        let tokens1 = limiter1.available_tokens().await;
        let tokens2 = limiter2.available_tokens().await;

        assert_eq!(tokens1, tokens2);
    }

    #[tokio::test]
    async fn test_global_registry_different_providers() {
        // Get limiters for different providers with unique names to avoid conflicts
        let provider1 = format!("test_provider_a_{}", uuid::Uuid::new_v4());
        let provider2 = format!("test_provider_b_{}", uuid::Uuid::new_v4());

        let limiter1 = global::get_rate_limiter(&provider1).await;
        let limiter2 = global::get_rate_limiter(&provider2).await;

        // They should have independent state
        limiter1.try_acquire().await;
        limiter1.try_acquire().await;

        let tokens1 = limiter1.available_tokens().await;
        let tokens2 = limiter2.available_tokens().await;

        // Second provider should still have more tokens than first
        assert!(tokens2 > tokens1);
    }

    #[tokio::test]
    async fn test_set_rate_limit() {
        global::set_rate_limit(
            "custom_provider",
            RateLimitConfig::new(120, 20),
        )
        .await;

        let limiter = global::get_rate_limiter("custom_provider").await;
        assert_eq!(limiter.config().requests_per_minute, 120);
        assert_eq!(limiter.config().burst_size, 20);
    }

    #[tokio::test]
    async fn test_disable_rate_limit() {
        global::disable_rate_limit("disabled_provider").await;

        let limiter = global::get_rate_limiter("disabled_provider").await;
        assert!(!limiter.is_enabled());

        // Should always succeed when disabled
        for _ in 0..100 {
            assert!(limiter.try_acquire().await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_clone_shares_state() {
        let limiter1 = RateLimiter::new(RateLimitConfig::new(60, 5));
        let limiter2 = limiter1.clone();

        // Consume tokens from limiter1
        limiter1.try_acquire().await;
        limiter1.try_acquire().await;

        // limiter2 should see the same state
        let tokens1 = limiter1.available_tokens().await;
        let tokens2 = limiter2.available_tokens().await;
        assert_eq!(tokens1, tokens2);
    }

    #[tokio::test]
    async fn test_rate_limiter_burst_size_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 600, // 10 per second
            burst_size: 3,
            enabled: true,
        });

        // Wait to ensure bucket is full
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Available tokens should not exceed burst size
        let available = limiter.available_tokens().await;
        assert_eq!(available, 3);
    }

    #[tokio::test]
    async fn test_rate_limiter_config_for_known_providers() {
        let providers = vec![
            "openai",
            "anthropic",
            "google",
            "azure",
            "doubao",
            "openrouter",
            "ollama",
            "glm",
        ];

        for provider in providers {
            let config = RateLimitConfig::for_provider(provider);
            assert!(config.enabled);
            assert!(config.requests_per_minute > 0);
            assert!(config.burst_size > 0);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_unknown_provider_uses_default() {
        let config = RateLimitConfig::for_provider("unknown_provider_xyz");
        let default_config = RateLimitConfig::default();

        assert_eq!(
            config.requests_per_minute,
            default_config.requests_per_minute
        );
        assert_eq!(config.burst_size, default_config.burst_size);
    }

    #[test]
    fn test_rate_limit_config_disabled() {
        let config = RateLimitConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_rate_limit_config_new() {
        let config = RateLimitConfig::new(100, 25);
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.burst_size, 25);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_rate_limiter_precise_timing() {
        // Test that refill happens correctly over time
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 600, // 10 tokens per second
            burst_size: 5,
            enabled: true,
        });

        // Exhaust all tokens
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }
        assert!(!limiter.try_acquire().await);

        // Wait for 300ms (should get ~3 tokens at 10/sec)
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Should have at least 2 tokens now
        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_acquire_returns_none_when_token_available() {
        let limiter = RateLimiter::new(RateLimitConfig::new(60, 10));

        // First acquire should not wait
        let wait_duration = limiter.acquire().await;
        assert!(wait_duration.is_none());
    }

    #[tokio::test]
    async fn test_available_tokens_after_partial_refill() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 600, // 10 per second
            burst_size: 10,
            enabled: true,
        });

        // Use all tokens
        for _ in 0..10 {
            limiter.try_acquire().await;
        }

        // Wait for partial refill (500ms = 5 tokens)
        tokio::time::sleep(Duration::from_millis(500)).await;

        let available = limiter.available_tokens().await;
        assert!(available >= 4 && available <= 6); // Allow some timing variance
    }
}
