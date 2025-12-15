//! Circuit breaker pattern for fault tolerance
//!
//! Prevents cascading failures by temporarily disabling failing operations.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

/// Circuit breaker for protecting against failing dependencies
pub struct CircuitBreaker {
    /// Component name (for logging and metrics)
    name: String,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Current state
    state: RwLock<CircuitState>,
    /// Failure count in current window
    failure_count: AtomicU32,
    /// Success count (for half-open state)
    success_count: AtomicU32,
    /// Time when circuit was opened
    opened_at: RwLock<Option<Instant>>,
    /// Active requests in half-open state
    half_open_requests: AtomicU32,
    /// Total calls counter
    total_calls: AtomicU64,
    /// Total failures counter
    total_failures: AtomicU64,
    /// Last failure time
    last_failure: RwLock<Option<Instant>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default config
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_config(name, CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom config
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            opened_at: RwLock::new(None),
            half_open_requests: AtomicU32::new(0),
            total_calls: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            last_failure: RwLock::new(None),
        }
    }

    /// Get the component name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current state
    pub async fn state(&self) -> CircuitState {
        // Check if we should transition from open to half-open
        let state = *self.state.read().await;
        if state == CircuitState::Open {
            if let Some(opened_at) = *self.opened_at.read().await {
                if opened_at.elapsed() >= self.config.reset_timeout {
                    self.transition_to_half_open().await;
                    return CircuitState::HalfOpen;
                }
            }
        }
        state
    }

    /// Check if the circuit allows operations
    pub async fn is_allowed(&self) -> bool {
        match self.state().await {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => {
                self.half_open_requests.load(Ordering::Acquire)
                    < self.config.half_open_max_requests
            }
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        let state = self.state().await;
        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                self.half_open_requests.fetch_sub(1, Ordering::Release);
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.config.success_threshold {
                    self.transition_to_closed().await;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        *self.last_failure.write().await = Some(Instant::now());

        let state = self.state().await;
        match state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if failures >= self.config.failure_threshold {
                    self.transition_to_open().await;
                }
            }
            CircuitState::HalfOpen => {
                self.half_open_requests.fetch_sub(1, Ordering::Release);
                // Any failure in half-open state opens the circuit again
                self.transition_to_open().await;
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Acquire permission to make a request (for half-open state tracking)
    pub async fn acquire(&self) -> bool {
        if self.state().await == CircuitState::HalfOpen {
            let current = self.half_open_requests.fetch_add(1, Ordering::AcqRel);
            if current >= self.config.half_open_max_requests {
                self.half_open_requests.fetch_sub(1, Ordering::Release);
                return false;
            }
        }
        self.is_allowed().await
    }

    /// Execute an operation with circuit breaker protection
    pub async fn call<T, E, F, Fut>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if !self.acquire().await {
            return Err(CircuitBreakerError::Open {
                component: self.name.clone(),
            });
        }

        match operation().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::OperationFailed(e))
            }
        }
    }

    /// Get circuit breaker statistics
    pub async fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: self.state().await,
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            last_failure: *self.last_failure.read().await,
            opened_at: *self.opened_at.read().await,
        }
    }

    /// Manually reset the circuit breaker to closed state
    pub async fn reset(&self) {
        self.transition_to_closed().await;
    }

    /// Manually open the circuit breaker
    pub async fn trip(&self) {
        self.transition_to_open().await;
    }

    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        *self.opened_at.write().await = Some(Instant::now());
        self.success_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);

        tracing::warn!(
            circuit = %self.name,
            "Circuit breaker opened after {} failures",
            self.failure_count.load(Ordering::Relaxed)
        );
    }

    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen;
        self.success_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);

        tracing::info!(
            circuit = %self.name,
            "Circuit breaker transitioning to half-open"
        );
    }

    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        *self.opened_at.write().await = None;

        tracing::info!(
            circuit = %self.name,
            "Circuit breaker closed"
        );
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

/// Collection of circuit breakers for multiple components
pub struct CircuitBreakerRegistry {
    breakers: dashmap::DashMap<String, Arc<CircuitBreaker>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// Create a new registry with default config
    pub fn new() -> Self {
        Self {
            breakers: dashmap::DashMap::new(),
            default_config: CircuitBreakerConfig::default(),
        }
    }

    /// Create a registry with custom default config
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: dashmap::DashMap::new(),
            default_config: config,
        }
    }

    /// Get or create a circuit breaker for a component
    pub fn get(&self, name: &str) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(name.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::with_config(name, self.default_config.clone()))
            })
            .clone()
    }

    /// Get or create with custom config
    pub fn get_with_config(&self, name: &str, config: CircuitBreakerConfig) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(CircuitBreaker::with_config(name, config)))
            .clone()
    }

    /// Get all circuit breaker names
    pub fn names(&self) -> Vec<String> {
        self.breakers.iter().map(|e| e.key().clone()).collect()
    }

    /// Get stats for all circuit breakers
    pub async fn all_stats(&self) -> Vec<(String, CircuitBreakerStats)> {
        let mut results = Vec::new();
        for entry in self.breakers.iter() {
            let stats = entry.value().stats().await;
            results.push((entry.key().clone(), stats));
        }
        results
    }

    /// Reset all circuit breakers
    pub async fn reset_all(&self) {
        for entry in self.breakers.iter() {
            entry.value().reset().await;
        }
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let result: Result<i32, CircuitBreakerError<&str>> =
            cb.call(|| async { Ok(42) }).await;

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

        let result: Result<i32, CircuitBreakerError<&str>> =
            cb.call(|| async { Ok(42) }).await;

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
