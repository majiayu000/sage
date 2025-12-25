//! Tests for circuit breaker functionality

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::super::breaker::CircuitBreaker;
    use super::super::registry::CircuitBreakerRegistry;
    use super::super::types::{CircuitBreakerConfig, CircuitBreakerError, CircuitState};

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let cb = CircuitBreaker::new("test");
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.is_allowed().await);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("test", config);

        // Record failures
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.is_allowed().await);
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("test", config);

        // Open the circuit
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be half-open now
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_closes_after_successes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            reset_timeout: Duration::from_millis(10),
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("test", config);

        // Open the circuit
        cb.record_failure().await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Now in half-open, record successes
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_call_success() {
        let cb = CircuitBreaker::new("test");

        let result: Result<i32, CircuitBreakerError<&str>> = cb.call(|| async { Ok(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_call_rejected_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config("test", config);

        // Open the circuit
        cb.record_failure().await;

        let result: Result<i32, CircuitBreakerError<&str>> = cb.call(|| async { Ok(42) }).await;

        assert!(matches!(result, Err(CircuitBreakerError::Open { .. })));
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = CircuitBreakerRegistry::new();

        let cb1 = registry.get("component_a");
        let cb2 = registry.get("component_b");
        let cb1_again = registry.get("component_a");

        // Should return same instance
        assert!(Arc::ptr_eq(&cb1, &cb1_again));
        assert!(!Arc::ptr_eq(&cb1, &cb2));

        let names = registry.names();
        assert!(names.contains(&"component_a".to_string()));
        assert!(names.contains(&"component_b".to_string()));
    }

    #[tokio::test]
    async fn test_stats() {
        let cb = CircuitBreaker::new("test");

        cb.record_success().await;
        cb.record_success().await;
        cb.record_failure().await;

        let stats = cb.stats().await;
        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.total_failures, 1);
        assert!((stats.failure_rate() - 33.33).abs() < 0.1);
    }
}
