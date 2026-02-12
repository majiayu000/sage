//! Circuit breaker pattern for fault tolerance
//!
//! Prevents cascading failures by temporarily disabling failing operations.

mod breaker;
mod types;

#[cfg(test)]
mod tests;

// Re-export all public items
pub use breaker::CircuitBreaker;
pub use types::{CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerStats, CircuitState};
