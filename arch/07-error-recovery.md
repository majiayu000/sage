# Error Recovery

## Overview

Sage Agent implements comprehensive error recovery mechanisms including retry policies, backoff strategies, circuit breakers, and task supervision.

## Error Classification

```rust
pub enum ErrorClass {
    Transient,   // May succeed on retry (network errors, rate limits)
    Permanent,   // Will not succeed on retry (invalid input, auth errors)
    Unknown,     // Attempt limited retries
}

pub fn classify_error(error: &SageError) -> ErrorClass {
    match error {
        SageError::Http(_) => ErrorClass::Transient,
        SageError::Timeout { .. } => ErrorClass::Transient,
        SageError::Config(_) => ErrorClass::Permanent,
        SageError::InvalidInput(_) => ErrorClass::Permanent,
        SageError::Cancelled => ErrorClass::Permanent,
        SageError::Llm(msg) => {
            if msg.contains("rate limit") || msg.contains("overloaded") {
                ErrorClass::Transient
            } else if msg.contains("invalid") || msg.contains("unauthorized") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Unknown
            }
        }
        _ => ErrorClass::Unknown,
    }
}
```

## Backoff Strategies

```
┌─────────────────────────────────────────────────────────────┐
│                   Backoff Strategies                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Exponential:   ▁▂▄█████████                               │
│                 base^attempt (with cap)                     │
│                                                              │
│  Linear:        ▁▂▃▄▅▆▇█                                   │
│                 base * attempt                              │
│                                                              │
│  Constant:      ████████                                    │
│                 same delay each time                        │
│                                                              │
│  Jitter:        ▁▃▂█▅▆▄▇                                   │
│                 randomized delays                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### BackoffStrategy Trait

```rust
pub trait BackoffStrategy: Send + Sync {
    fn delay_for_attempt(&self, attempt: u32) -> Duration;
    fn reset(&mut self);
}
```

### Implementations

```rust
// Exponential backoff: base_delay * 2^attempt (capped)
pub struct ExponentialBackoff {
    base_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
}

impl ExponentialBackoff {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self;
    pub fn with_multiplier(self, multiplier: f64) -> Self;
}

// Constant delay
pub struct ConstantBackoff {
    delay: Duration,
}

// Linear backoff: base_delay * attempt
pub struct LinearBackoff {
    base_delay: Duration,
    max_delay: Duration,
}

// Decorrelated jitter (AWS style)
pub struct DecorrelatedJitterBackoff {
    base_delay: Duration,
    max_delay: Duration,
    last_delay: AtomicU64,
}
```

## Retry Policy

```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub retry_transient: bool,
    pub retry_unknown: bool,
    pub retry_permanent: bool,
}

pub struct RetryPolicy {
    config: RetryConfig,
    backoff: Box<dyn BackoffStrategy>,
}

impl RetryPolicy {
    pub fn new(config: RetryConfig, backoff: Box<dyn BackoffStrategy>) -> Self;

    pub fn should_retry(&self, error: &SageError, attempt: u32) -> bool {
        if attempt + 1 >= self.config.max_attempts {
            return false;
        }

        let class = classify_error(error);
        match class {
            ErrorClass::Transient => self.config.retry_transient,
            ErrorClass::Permanent => self.config.retry_permanent,
            ErrorClass::Unknown => self.config.retry_unknown,
        }
    }

    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        self.backoff.delay_for_attempt(attempt)
    }

    pub async fn execute<F, T, E>(&self, operation: F) -> RetryResult<T, E>
    where
        F: Fn() -> Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
        E: Into<SageError>,
    {
        // Execute with retry logic
    }
}

pub enum RetryResult<T, E> {
    Success(T),
    Failed { last_error: E, attempts: u32 },
    MaxAttemptsExceeded { last_error: E, attempts: u32 },
}
```

### Usage Example

```rust
let policy = RetryPolicy::new(
    RetryConfig {
        max_attempts: 3,
        retry_transient: true,
        retry_unknown: true,
        retry_permanent: false,
    },
    Box::new(ExponentialBackoff::new(
        Duration::from_millis(100),
        Duration::from_secs(10),
    )),
);

let result = policy.execute(|| {
    Box::pin(async {
        llm_client.chat(&messages, tools).await
    })
}).await;
```

## Circuit Breaker

```
┌─────────────────────────────────────────────────────────────┐
│                  Circuit Breaker States                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│                    ┌─────────────┐                          │
│         ┌─────────▶│   CLOSED    │◀─────────┐              │
│         │          │  (normal)   │          │              │
│         │          └──────┬──────┘          │              │
│         │                 │                  │              │
│         │    failure_count >= threshold     │              │
│         │                 │                  │              │
│         │          ┌──────▼──────┐          │              │
│         │          │    OPEN     │          │              │
│   success          │  (failing)  │      success            │
│   in half-open     └──────┬──────┘      count >= threshold │
│         │                 │                  │              │
│         │         timeout elapsed           │              │
│         │                 │                  │              │
│         │          ┌──────▼──────┐          │              │
│         └──────────│  HALF-OPEN  │──────────┘              │
│                    │  (testing)  │                         │
│                    └─────────────┘                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Rejecting requests
    HalfOpen,  // Testing recovery
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
}

pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: RwLock<Option<Instant>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self;

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        // Check if circuit is open
        if self.is_open().await {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        match operation.await {
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

    pub async fn state(&self) -> CircuitState;
    async fn is_open(&self) -> bool;
    async fn record_success(&self);
    async fn record_failure(&self);
}
```

### Circuit Breaker Registry

```rust
pub struct CircuitBreakerRegistry {
    breakers: DashMap<String, Arc<CircuitBreaker>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    pub fn new(default_config: CircuitBreakerConfig) -> Self;
    pub fn get_or_create(&self, name: &str) -> Arc<CircuitBreaker>;
    pub fn get(&self, name: &str) -> Option<Arc<CircuitBreaker>>;
    pub async fn states(&self) -> HashMap<String, CircuitState>;
}
```

## Task Supervisor

```rust
pub enum SupervisionPolicy {
    Restart { max_restarts: u32, window: Duration },
    Resume,
    Stop,
    Escalate,
}

pub enum SupervisionResult<T> {
    Completed(T),
    Restarted { attempts: u32, last_error: SageError },
    Stopped { error: SageError },
    Escalated { error: SageError },
}

pub struct TaskSupervisor {
    policy: SupervisionPolicy,
    restart_count: AtomicU32,
    window_start: RwLock<Instant>,
}

impl TaskSupervisor {
    pub fn new(policy: SupervisionPolicy) -> Self;

    pub async fn supervise<F, T>(&self, task: F) -> SupervisionResult<T>
    where
        F: Fn() -> Pin<Box<dyn Future<Output = Result<T, SageError>> + Send>> + Sync,
    {
        loop {
            match task().await {
                Ok(result) => return SupervisionResult::Completed(result),
                Err(error) => {
                    match self.handle_failure(&error).await {
                        FailureAction::Restart => continue,
                        FailureAction::Stop => return SupervisionResult::Stopped { error },
                        FailureAction::Escalate => return SupervisionResult::Escalated { error },
                    }
                }
            }
        }
    }
}
```

### Supervisor

```rust
pub struct Supervisor {
    name: String,
    default_policy: SupervisionPolicy,
    children: DashMap<String, TaskSupervisor>,
}

impl Supervisor {
    pub fn new(name: impl Into<String>, policy: SupervisionPolicy) -> Self;
    pub fn create_child(&self, name: &str) -> Arc<TaskSupervisor>;
    pub fn create_child_with_policy(&self, name: &str, policy: SupervisionPolicy) -> Arc<TaskSupervisor>;

    pub async fn spawn<F, T>(&self, name: &str, task: F) -> JoinHandle<SupervisionResult<T>>
    where
        F: Fn() -> Pin<Box<dyn Future<Output = Result<T, SageError>> + Send>> + Send + Sync + 'static,
        T: Send + 'static;
}
```

## Error Recovery Flow

```
┌─────────────────────────────────────────────────────────────┐
│                   Error Recovery Flow                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   Operation                                                  │
│       │                                                      │
│       ▼                                                      │
│   ┌───────────┐                                             │
│   │  Execute  │                                             │
│   └─────┬─────┘                                             │
│         │                                                    │
│    ┌────┴────┐                                              │
│    │         │                                               │
│    ▼         ▼                                               │
│ Success    Error                                             │
│    │         │                                               │
│    │    ┌────▼────────┐                                     │
│    │    │  Classify   │                                     │
│    │    │   Error     │                                     │
│    │    └──────┬──────┘                                     │
│    │           │                                             │
│    │    ┌──────┴──────┬────────────┐                        │
│    │    │             │            │                         │
│    │    ▼             ▼            ▼                         │
│    │ Transient    Unknown     Permanent                     │
│    │    │             │            │                         │
│    │    │    ┌────────┴────────┐   │                        │
│    │    │    │                 │   │                        │
│    │    ▼    ▼                 │   │                        │
│    │  ┌──────────────┐        │   │                        │
│    │  │Circuit Breaker│       │   │                        │
│    │  │    Check     │        │   │                        │
│    │  └───────┬──────┘        │   │                        │
│    │          │               │   │                        │
│    │     ┌────┴────┐          │   │                        │
│    │     │         │          │   │                        │
│    │     ▼         ▼          │   │                        │
│    │   Open      Closed       │   │                        │
│    │     │         │          │   │                        │
│    │     │    ┌────▼────┐     │   │                        │
│    │     │    │  Retry  │     │   │                        │
│    │     │    │  Policy │     │   │                        │
│    │     │    └────┬────┘     │   │                        │
│    │     │         │          │   │                        │
│    │     │    ┌────┴────┐     │   │                        │
│    │     │    │         │     │   │                        │
│    │     │    ▼         ▼     │   │                        │
│    │     │  Retry     No      │   │                        │
│    │     │    │      Retry    │   │                        │
│    │     │    │         │     │   │                        │
│    │     │    └─►Execute│◄────┘   │                        │
│    │     │              │         │                        │
│    │     └──────┬───────┴─────────┘                        │
│    │            │                                           │
│    │       ┌────▼─────┐                                    │
│    │       │Supervisor│                                    │
│    │       │  Policy  │                                    │
│    │       └────┬─────┘                                    │
│    │            │                                           │
│    │   ┌────────┴────────┬────────────┐                    │
│    │   │                 │            │                     │
│    │   ▼                 ▼            ▼                     │
│    │ Restart           Resume       Stop                   │
│    │   │                 │            │                     │
│    │   └────►Execute     │            │                     │
│    │                     │            │                     │
│    └─────────────────────┴────────────┴────► Result        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```
