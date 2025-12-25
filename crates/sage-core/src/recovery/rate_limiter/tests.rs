//! Tests for rate limiting

#[cfg(test)]
mod tests {
    use crate::recovery::rate_limiter::{
        RateLimitError, RateLimiter, RateLimiterConfig, SlidingWindowRateLimiter,
    };
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(100.0)
                .with_burst_size(5),
        );

        // Should be able to acquire burst_size tokens immediately
        for _ in 0..5 {
            assert!(limiter.try_acquire().await.is_some());
        }

        // Should be rate limited now
        assert!(limiter.try_acquire().await.is_none());
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(100.0)
                .with_burst_size(2),
        );

        // Use all tokens
        limiter.try_acquire().await;
        limiter.try_acquire().await;

        // Should be limited
        assert!(limiter.try_acquire().await.is_none());

        // Wait for refill
        sleep(Duration::from_millis(20)).await;

        // Should have tokens again
        assert!(limiter.try_acquire().await.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocking() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(1000.0)
                .with_burst_size(1),
        );

        // Use the token
        limiter.try_acquire().await;

        // Blocking acquire should eventually succeed
        let start = Instant::now();
        let result = limiter.acquire().await;
        assert!(result.is_ok());
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_rate_limiter_non_blocking() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(1.0)
                .with_burst_size(1)
                .non_blocking(),
        );

        // Use the token
        limiter.try_acquire().await;

        // Non-blocking acquire should fail immediately
        let result = limiter.acquire().await;
        assert!(matches!(result, Err(RateLimitError::WouldBlock)));
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrency() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(1000.0)
                .with_burst_size(100)
                .with_max_concurrent(2)
                .non_blocking(),
        );

        // Acquire two permits
        let g1 = limiter.acquire().await.unwrap();
        let g2 = limiter.acquire().await.unwrap();

        // Third should fail due to concurrency
        let result = limiter.acquire().await;
        assert!(matches!(
            result,
            Err(RateLimitError::ConcurrencyExceeded { .. })
        ));

        // Drop one permit
        drop(g1);

        // Should succeed now
        let _g3 = limiter.acquire().await.unwrap();

        drop(g2);
    }

    #[tokio::test]
    async fn test_rate_limiter_available_tokens() {
        let limiter = RateLimiter::with_config(
            RateLimiterConfig::default()
                .with_rps(10.0)
                .with_burst_size(10),
        );

        let initial = limiter.available_tokens().await;
        assert!((initial - 10.0).abs() < 0.01);

        limiter.try_acquire().await;
        let after_one = limiter.available_tokens().await;
        assert!(after_one < initial);
    }

    #[tokio::test]
    async fn test_sliding_window_basic() {
        let limiter = SlidingWindowRateLimiter::new(3, Duration::from_millis(100));

        // Should allow 3 requests
        assert!(limiter.try_record().await);
        assert!(limiter.try_record().await);
        assert!(limiter.try_record().await);

        // Fourth should fail
        assert!(!limiter.try_record().await);

        // Wait for window to pass
        sleep(Duration::from_millis(110)).await;

        // Should allow again
        assert!(limiter.try_record().await);
    }

    #[tokio::test]
    async fn test_sliding_window_per_second() {
        let limiter = SlidingWindowRateLimiter::per_second(2);

        assert!(limiter.try_record().await);
        assert!(limiter.try_record().await);
        assert!(!limiter.try_record().await);

        assert!(limiter.is_limited().await);
    }

    #[tokio::test]
    async fn test_sliding_window_blocking() {
        let limiter = SlidingWindowRateLimiter::new(1, Duration::from_millis(50))
            .with_max_wait(Duration::from_secs(1));

        // Use the slot
        limiter.record().await.unwrap();

        // Blocking record should eventually succeed
        let start = Instant::now();
        limiter.record().await.unwrap();
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_sliding_window_current_count() {
        let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(10));

        assert_eq!(limiter.current_count().await, 0);

        limiter.try_record().await;
        limiter.try_record().await;

        assert_eq!(limiter.current_count().await, 2);
    }

    #[test]
    fn test_rate_limit_error_display() {
        let timeout = RateLimitError::Timeout {
            waited: Duration::from_secs(30),
        };
        assert!(timeout.to_string().contains("30"));

        let concurrency = RateLimitError::ConcurrencyExceeded { max: 5 };
        assert!(concurrency.to_string().contains("5"));
    }

    #[test]
    fn test_config_presets() {
        let anthropic = RateLimiterConfig::for_anthropic();
        assert!(anthropic.requests_per_second >= 50.0);

        let openai = RateLimiterConfig::for_openai();
        assert!(openai.requests_per_second >= 60.0);

        let conservative = RateLimiterConfig::conservative();
        assert!(conservative.requests_per_second <= 2.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent_requests() {
        let limiter = RateLimiter::with_config(RateLimiterConfig::default().with_max_concurrent(3));

        assert_eq!(limiter.concurrent_requests(), 0);

        let _g1 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.concurrent_requests(), 1);

        let _g2 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.concurrent_requests(), 2);
    }
}
