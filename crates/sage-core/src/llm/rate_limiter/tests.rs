//! Tests for rate limiter

use super::bucket::RateLimiter;
use super::limiter;
use super::types::RateLimitConfig;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_rate_limiter_allows_burst() {
    let limiter = RateLimiter::new(RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 5,
        max_concurrent: 0, // unlimited
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
        max_concurrent: 0, // unlimited
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
        max_concurrent: 0, // unlimited
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
    assert!(openai.max_concurrent >= 5);

    let anthropic = RateLimitConfig::for_provider("anthropic");
    assert!(anthropic.requests_per_minute >= 60);
    assert!(anthropic.max_concurrent >= 5);

    let ollama = RateLimitConfig::for_provider("ollama");
    assert!(ollama.requests_per_minute >= 60);
    assert!(ollama.max_concurrent >= 10); // Local providers can handle more
}

#[tokio::test]
async fn test_acquire_waits() {
    let limiter = RateLimiter::new(RateLimitConfig {
        requests_per_minute: 600, // 10 per second for faster test
        burst_size: 1,
        max_concurrent: 0, // unlimited
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
    let limiter1 = limiter::get_rate_limiter("test_provider").await;
    let limiter2 = limiter::get_rate_limiter("test_provider").await;

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

    let limiter1 = limiter::get_rate_limiter(&provider1).await;
    let limiter2 = limiter::get_rate_limiter(&provider2).await;

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
    limiter::set_rate_limit("custom_provider", RateLimitConfig::new(120, 20)).await;

    let limiter = limiter::get_rate_limiter("custom_provider").await;
    assert_eq!(limiter.config().requests_per_minute, 120);
    assert_eq!(limiter.config().burst_size, 20);
}

#[tokio::test]
async fn test_disable_rate_limit() {
    limiter::disable_rate_limit("disabled_provider").await;

    let limiter = limiter::get_rate_limiter("disabled_provider").await;
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
        max_concurrent: 0, // unlimited
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
        assert!(config.max_concurrent > 0);
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
    assert_eq!(config.max_concurrent, default_config.max_concurrent);
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
        max_concurrent: 0, // unlimited
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
        max_concurrent: 0, // unlimited
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

// New tests for concurrent request limiting

#[tokio::test]
async fn test_concurrent_requests_tracking() {
    let limiter = RateLimiter::new(RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 10,
        max_concurrent: 5,
        enabled: true,
    });

    // Initially should have 0 concurrent requests
    assert_eq!(limiter.concurrent_requests(), 0);

    // max_concurrent should match config
    assert_eq!(limiter.max_concurrent(), 5);
}

#[tokio::test]
async fn test_with_concurrent_constructor() {
    let config = RateLimitConfig::with_concurrent(120, 25, 10);
    assert_eq!(config.requests_per_minute, 120);
    assert_eq!(config.burst_size, 25);
    assert_eq!(config.max_concurrent, 10);
    assert!(config.enabled);
}

#[tokio::test]
async fn test_rate_limiter_clone_shares_concurrent_state() {
    let limiter1 = RateLimiter::new(RateLimitConfig::with_concurrent(60, 5, 10));
    let limiter2 = limiter1.clone();

    // Both should share the same concurrent semaphore
    // After consuming from limiter1, limiter2 should see the same state
    limiter1.try_acquire().await;

    let tokens1 = limiter1.available_tokens().await;
    let tokens2 = limiter2.available_tokens().await;
    assert_eq!(tokens1, tokens2);
}

#[test]
fn test_rate_limit_config_default_includes_concurrent() {
    let config = RateLimitConfig::default();
    assert!(config.max_concurrent > 0);
    assert_eq!(config.max_concurrent, 5); // Default should be 5
}

#[test]
fn test_rate_limit_config_disabled_includes_concurrent() {
    let config = RateLimitConfig::disabled();
    assert!(!config.enabled);
    // Should still have a max_concurrent value even when disabled
    assert!(config.max_concurrent > 0);
}
