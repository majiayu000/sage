//! Type definitions for fallback system

use std::time::{Duration, Instant};

/// Reason for falling back to another model
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackReason {
    /// Primary model rate limited
    RateLimited,
    /// Primary model unavailable
    Unavailable,
    /// Primary model returned error
    Error(String),
    /// Primary model timed out
    Timeout,
    /// Cost limit exceeded
    CostLimit,
    /// Context length exceeded
    ContextTooLong,
    /// Manual fallback requested
    Manual,
}

impl std::fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited => write!(f, "rate limited"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::Error(e) => write!(f, "error: {}", e),
            Self::Timeout => write!(f, "timeout"),
            Self::CostLimit => write!(f, "cost limit exceeded"),
            Self::ContextTooLong => write!(f, "context too long"),
            Self::Manual => write!(f, "manual fallback"),
        }
    }
}

/// Configuration for a model in the fallback chain
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model identifier
    pub model_id: String,
    /// Provider name
    pub provider: String,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Maximum context window
    pub max_context: usize,
    /// Whether this model is currently healthy
    pub healthy: bool,
    /// Cooldown duration after failure
    pub cooldown: Duration,
    /// Maximum retries before fallback
    pub max_retries: u32,
}

impl ModelConfig {
    /// Create a new model config
    pub fn new(model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            provider: provider.into(),
            priority: 0,
            max_context: 128_000,
            healthy: true,
            cooldown: Duration::from_secs(60),
            max_retries: 2,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set max context
    pub fn with_max_context(mut self, max: usize) -> Self {
        self.max_context = max;
        self
    }

    /// Set cooldown duration
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }
}

/// Statistics for a model
#[derive(Debug, Clone)]
pub struct ModelStats {
    /// Model identifier
    pub model_id: String,
    /// Provider name
    pub provider: String,
    /// Whether model is available
    pub available: bool,
    /// Total requests made
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Current consecutive failures
    pub failure_count: u32,
}

/// Record of a fallback event
#[derive(Debug, Clone)]
pub struct FallbackEvent {
    /// Model we fell back from
    pub from_model: String,
    /// Model we fell back to
    pub to_model: Option<String>,
    /// Reason for fallback
    pub reason: FallbackReason,
    /// When the fallback occurred
    pub timestamp: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_reason_display() {
        assert_eq!(FallbackReason::RateLimited.to_string(), "rate limited");
        assert_eq!(FallbackReason::Timeout.to_string(), "timeout");
        assert!(
            FallbackReason::Error("test".into())
                .to_string()
                .contains("test")
        );
    }

    #[test]
    fn test_fallback_reason_equality() {
        assert_eq!(FallbackReason::RateLimited, FallbackReason::RateLimited);
        assert_ne!(FallbackReason::RateLimited, FallbackReason::Timeout);

        let err1 = FallbackReason::Error("test".into());
        let err2 = FallbackReason::Error("test".into());
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_model_config_builder() {
        let config = ModelConfig::new("test", "provider")
            .with_priority(5)
            .with_max_context(50000)
            .with_cooldown(Duration::from_secs(30))
            .with_max_retries(3);

        assert_eq!(config.priority, 5);
        assert_eq!(config.max_context, 50000);
        assert_eq!(config.cooldown, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_model_config_defaults() {
        let config = ModelConfig::new("test", "provider");
        assert_eq!(config.priority, 0);
        assert_eq!(config.max_context, 128_000);
        assert!(config.healthy);
        assert_eq!(config.cooldown, Duration::from_secs(60));
        assert_eq!(config.max_retries, 2);
    }
}
