//! Global rate limiter registry for per-provider rate limiting

use super::bucket::RateLimiter;
use super::types::RateLimitConfig;
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
