//! Circuit breaker registry for managing multiple circuit breakers

use std::sync::Arc;

use super::breaker::CircuitBreaker;
use super::types::{CircuitBreakerConfig, CircuitBreakerStats};

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
                Arc::new(CircuitBreaker::with_config(
                    name,
                    self.default_config.clone(),
                ))
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
