//! Circuit breaker implementation

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;

use super::types::{CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerStats, CircuitState};

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
                self.half_open_requests.load(Ordering::Acquire) < self.config.half_open_max_requests
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
