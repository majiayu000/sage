//! Builder for creating fallback chains

use std::sync::Arc;
use tokio::sync::RwLock;

use super::manager::FallbackChain;
use super::types::ModelConfig;

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
    #[allow(clippy::should_implement_trait)]
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
    async fn test_default_builder() {
        let builder = FallbackChainBuilder::default();
        let chain = builder.build().await;
        assert!(chain.is_empty().await);
    }

    #[tokio::test]
    async fn test_builder_add_method() {
        let config = ModelConfig::new("test", "provider");
        let chain = FallbackChainBuilder::new().add(config).build().await;

        assert_eq!(chain.model_count().await, 1);
    }
}
