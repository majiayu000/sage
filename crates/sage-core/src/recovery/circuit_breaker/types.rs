//! Circuit breaker types and configuration

use std::time::{Duration, Instant};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, operations proceed normally
    Closed,
    /// Circuit is open, operations are rejected
    Open,
    /// Circuit is half-open, limited operations allowed to test recovery
    HalfOpen,
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of successes needed in half-open state to close
    pub success_threshold: u32,
    /// Time to wait before transitioning from open to half-open
    pub reset_timeout: Duration,
    /// Sliding window size for failure counting
    pub window_size: Duration,
    /// Maximum concurrent requests in half-open state
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            window_size: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a config for aggressive circuit breaking
    pub fn aggressive() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            reset_timeout: Duration::from_secs(15),
            window_size: Duration::from_secs(30),
            half_open_max_requests: 1,
        }
    }

    /// Create a config for lenient circuit breaking
    pub fn lenient() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 5,
            reset_timeout: Duration::from_secs(60),
            window_size: Duration::from_secs(120),
            half_open_max_requests: 5,
        }
    }
}

/// Error from circuit breaker operations
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open
    Open { component: String },
    /// Operation failed
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { component } => {
                write!(f, "Circuit breaker open for component: {}", component)
            }
            Self::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error> std::error::Error for CircuitBreakerError<E> {}

/// Statistics for a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub total_calls: u64,
    pub total_failures: u64,
    pub last_failure: Option<Instant>,
    pub opened_at: Option<Instant>,
}

impl CircuitBreakerStats {
    /// Calculate failure rate as a percentage
    pub fn failure_rate(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            (self.total_failures as f64 / self.total_calls as f64) * 100.0
        }
    }
}
