//! Model fallback chain
//!
//! This module provides automatic fallback to alternative models
//! when the primary model fails or is rate limited.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

/// State tracking for a model
#[derive(Debug, Clone)]
struct ModelState {
    /// Configuration
    config: ModelConfig,
    /// Last failure time
    last_failure: Option<Instant>,
    /// Consecutive failure count
    failure_count: u32,
    /// Total requests
    total_requests: u64,
    /// Successful requests
    successful_requests: u64,
}

impl ModelState {
    fn new(config: ModelConfig) -> Self {
        Self {
            config,
            last_failure: None,
            failure_count: 0,
            total_requests: 0,
            successful_requests: 0,
        }
    }

    fn is_available(&self) -> bool {
        if !self.config.healthy {
            return false;
        }

        match self.last_failure {
            Some(time) => time.elapsed() > self.config.cooldown,
            None => true,
        }
    }

    fn record_success(&mut self) {
        self.failure_count = 0;
        self.total_requests += 1;
        self.successful_requests += 1;
    }

    fn record_failure(&mut self) {
        self.failure_count += 1;
        self.total_requests += 1;
        self.last_failure = Some(Instant::now());
    }

    fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }
}

/// Model fallback chain manager
#[derive(Debug)]
pub struct FallbackChain {
    /// Models in the chain
    models: Arc<RwLock<Vec<ModelState>>>,
    /// Current model index
    current_index: Arc<RwLock<usize>>,
    /// Fallback history
    history: Arc<RwLock<Vec<FallbackEvent>>>,
    /// Maximum history entries
    max_history: usize,
}

impl FallbackChain {
    /// Create a new fallback chain
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// Add a model to the chain
    pub async fn add_model(&self, config: ModelConfig) {
        let mut models = self.models.write().await;
        let state = ModelState::new(config);

        // Insert in priority order
        let pos = models
            .iter()
            .position(|m| m.config.priority > state.config.priority)
            .unwrap_or(models.len());

        models.insert(pos, state);
    }

    /// Get the current active model
    pub async fn current_model(&self) -> Option<String> {
        let models = self.models.read().await;
        let index = *self.current_index.read().await;

        models.get(index).map(|m| m.config.model_id.clone())
    }

    /// Get next available model
    pub async fn next_available(&self, context_size: Option<usize>) -> Option<String> {
        let models = self.models.read().await;

        for (i, state) in models.iter().enumerate() {
            if !state.is_available() {
                continue;
            }

            // Check context size if provided
            if let Some(size) = context_size {
                if size > state.config.max_context {
                    continue;
                }
            }

            // Update current index
            let mut current = self.current_index.write().await;
            *current = i;

            return Some(state.config.model_id.clone());
        }

        None
    }

    /// Record a successful request
    pub async fn record_success(&self, model_id: &str) {
        let mut models = self.models.write().await;

        if let Some(state) = models.iter_mut().find(|m| m.config.model_id == model_id) {
            state.record_success();
        }
    }

    /// Record a failed request and potentially trigger fallback
    pub async fn record_failure(
        &self,
        model_id: &str,
        reason: FallbackReason,
    ) -> Option<String> {
        let mut models = self.models.write().await;

        // Find and update the failed model
        let failed_index = models
            .iter()
            .position(|m| m.config.model_id == model_id);

        if let Some(index) = failed_index {
            models[index].record_failure();

            // Record fallback event
            let event = FallbackEvent {
                from_model: model_id.to_string(),
                to_model: None,
                reason: reason.clone(),
                timestamp: Instant::now(),
            };
            self.add_history_event(event).await;

            // Check if we should fallback
            if models[index].failure_count >= models[index].config.max_retries {
                // Find next available model
                drop(models);
                return self.next_available(None).await;
            }
        }

        None
    }

    /// Force fallback to next model
    pub async fn force_fallback(&self, reason: FallbackReason) -> Option<String> {
        let current = self.current_model().await?;

        let current_index = self.current_index.write().await;
        let models = self.models.read().await;

        // Find next available model after current
        for i in (*current_index + 1)..models.len() {
            if models[i].is_available() {
                let new_model = models[i].config.model_id.clone();

                // Record event
                let event = FallbackEvent {
                    from_model: current.clone(),
                    to_model: Some(new_model.clone()),
                    reason,
                    timestamp: Instant::now(),
                };
                drop(models);
                drop(current_index);
                self.add_history_event(event).await;

                let mut current_index = self.current_index.write().await;
                *current_index = i;
                return Some(new_model);
            }
        }

        None
    }

    /// Reset a model to healthy state
    pub async fn reset_model(&self, model_id: &str) {
        let mut models = self.models.write().await;

        if let Some(state) = models.iter_mut().find(|m| m.config.model_id == model_id) {
            state.failure_count = 0;
            state.last_failure = None;
            state.config.healthy = true;
        }
    }

    /// Reset all models
    pub async fn reset_all(&self) {
        let mut models = self.models.write().await;

        for state in models.iter_mut() {
            state.failure_count = 0;
            state.last_failure = None;
            state.config.healthy = true;
        }

        *self.current_index.write().await = 0;
    }

    /// Get model statistics
    pub async fn get_stats(&self) -> Vec<ModelStats> {
        let models = self.models.read().await;

        models
            .iter()
            .map(|m| ModelStats {
                model_id: m.config.model_id.clone(),
                provider: m.config.provider.clone(),
                available: m.is_available(),
                total_requests: m.total_requests,
                successful_requests: m.successful_requests,
                success_rate: m.success_rate(),
                failure_count: m.failure_count,
            })
            .collect()
    }

    /// Get fallback history
    pub async fn get_history(&self) -> Vec<FallbackEvent> {
        self.history.read().await.clone()
    }

    /// Add history event
    async fn add_history_event(&self, event: FallbackEvent) {
        let mut history = self.history.write().await;
        history.push(event);

        // Trim to max size
        while history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Get model count
    pub async fn model_count(&self) -> usize {
        self.models.read().await.len()
    }

    /// Check if chain is empty
    pub async fn is_empty(&self) -> bool {
        self.models.read().await.is_empty()
    }

    /// List all models
    pub async fn list_models(&self) -> Vec<String> {
        self.models
            .read()
            .await
            .iter()
            .map(|m| m.config.model_id.clone())
            .collect()
    }
}

impl Default for FallbackChain {
    fn default() -> Self {
        Self::new()
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

/// Builder for creating fallback chains
pub struct FallbackChainBuilder {
    models: Vec<ModelConfig>,
    max_history: usize,
}

impl FallbackChainBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            max_history: 100,
        }
    }

    /// Add a model to the chain
    pub fn add(mut self, config: ModelConfig) -> Self {
        self.models.push(config);
        self
    }

    /// Add a model by ID with default config
    pub fn add_model(mut self, model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        self.models.push(ModelConfig::new(model_id, provider));
        self
    }

    /// Set max history size
    pub fn max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Build the fallback chain
    pub async fn build(self) -> FallbackChain {
        let chain = FallbackChain {
            models: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: self.max_history,
        };

        for config in self.models {
            chain.add_model(config).await;
        }

        chain
    }
}

impl Default for FallbackChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default fallback chain for Anthropic
pub async fn anthropic_fallback_chain() -> FallbackChain {
    FallbackChainBuilder::new()
        .add(
            ModelConfig::new("claude-3-5-sonnet-20241022", "anthropic")
                .with_priority(0)
                .with_max_context(200_000),
        )
        .add(
            ModelConfig::new("claude-3-5-haiku-20241022", "anthropic")
                .with_priority(1)
                .with_max_context(200_000),
        )
        .add(
            ModelConfig::new("claude-3-opus-20240229", "anthropic")
                .with_priority(2)
                .with_max_context(200_000),
        )
        .build()
        .await
}

/// Create a default fallback chain for OpenAI
pub async fn openai_fallback_chain() -> FallbackChain {
    FallbackChainBuilder::new()
        .add(
            ModelConfig::new("gpt-4o", "openai")
                .with_priority(0)
                .with_max_context(128_000),
        )
        .add(
            ModelConfig::new("gpt-4o-mini", "openai")
                .with_priority(1)
                .with_max_context(128_000),
        )
        .add(
            ModelConfig::new("gpt-4-turbo", "openai")
                .with_priority(2)
                .with_max_context(128_000),
        )
        .build()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_chain_creation() {
        let chain = FallbackChain::new();
        assert!(chain.is_empty().await);
    }

    #[tokio::test]
    async fn test_add_model() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "provider1")).await;

        assert_eq!(chain.model_count().await, 1);
        assert_eq!(chain.current_model().await, Some("model1".to_string()));
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let chain = FallbackChain::new();

        chain.add_model(ModelConfig::new("low", "p").with_priority(10)).await;
        chain.add_model(ModelConfig::new("high", "p").with_priority(1)).await;
        chain.add_model(ModelConfig::new("medium", "p").with_priority(5)).await;

        let models = chain.list_models().await;
        assert_eq!(models, vec!["high", "medium", "low"]);
    }

    #[tokio::test]
    async fn test_record_success() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "provider1")).await;

        chain.record_success("model1").await;

        let stats = chain.get_stats().await;
        assert_eq!(stats[0].total_requests, 1);
        assert_eq!(stats[0].successful_requests, 1);
    }

    #[tokio::test]
    async fn test_record_failure_triggers_fallback() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "p").with_max_retries(1)).await;
        chain.add_model(ModelConfig::new("model2", "p")).await;

        // First failure
        let next = chain.record_failure("model1", FallbackReason::RateLimited).await;
        assert_eq!(next, Some("model2".to_string()));
    }

    #[tokio::test]
    async fn test_force_fallback() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "p")).await;
        chain.add_model(ModelConfig::new("model2", "p")).await;

        let next = chain.force_fallback(FallbackReason::Manual).await;
        assert_eq!(next, Some("model2".to_string()));
        assert_eq!(chain.current_model().await, Some("model2".to_string()));
    }

    #[tokio::test]
    async fn test_reset_model() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "p")).await;

        chain.record_failure("model1", FallbackReason::Error("test".into())).await;
        chain.reset_model("model1").await;

        let stats = chain.get_stats().await;
        assert_eq!(stats[0].failure_count, 0);
    }

    #[tokio::test]
    async fn test_reset_all() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "p")).await;
        chain.add_model(ModelConfig::new("model2", "p")).await;

        chain.force_fallback(FallbackReason::Manual).await;
        chain.reset_all().await;

        assert_eq!(chain.current_model().await, Some("model1".to_string()));
    }

    #[tokio::test]
    async fn test_context_size_filtering() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("small", "p").with_max_context(1000)).await;
        chain.add_model(ModelConfig::new("large", "p").with_max_context(100000)).await;

        // Request too large for first model
        let model = chain.next_available(Some(50000)).await;
        assert_eq!(model, Some("large".to_string()));
    }

    #[tokio::test]
    async fn test_fallback_history() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "p").with_max_retries(0)).await;
        chain.add_model(ModelConfig::new("model2", "p")).await;

        chain.record_failure("model1", FallbackReason::Timeout).await;

        let history = chain.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_model, "model1");
    }

    #[tokio::test]
    async fn test_builder() {
        let chain = FallbackChainBuilder::new()
            .add_model("model1", "provider1")
            .add_model("model2", "provider2")
            .max_history(50)
            .build()
            .await;

        assert_eq!(chain.model_count().await, 2);
    }

    #[tokio::test]
    async fn test_anthropic_chain() {
        let chain = anthropic_fallback_chain().await;
        assert_eq!(chain.model_count().await, 3);
        assert!(chain.current_model().await.unwrap().contains("sonnet"));
    }

    #[tokio::test]
    async fn test_openai_chain() {
        let chain = openai_fallback_chain().await;
        assert_eq!(chain.model_count().await, 3);
        assert!(chain.current_model().await.unwrap().contains("gpt-4o"));
    }

    #[tokio::test]
    async fn test_model_stats() {
        let chain = FallbackChain::new();
        chain.add_model(ModelConfig::new("model1", "provider1")).await;

        chain.record_success("model1").await;
        chain.record_success("model1").await;
        chain.record_failure("model1", FallbackReason::Timeout).await;

        let stats = chain.get_stats().await;
        assert_eq!(stats[0].total_requests, 3);
        assert_eq!(stats[0].successful_requests, 2);
        assert!((stats[0].success_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_fallback_reason_display() {
        assert_eq!(FallbackReason::RateLimited.to_string(), "rate limited");
        assert_eq!(FallbackReason::Timeout.to_string(), "timeout");
        assert!(FallbackReason::Error("test".into()).to_string().contains("test"));
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
}
