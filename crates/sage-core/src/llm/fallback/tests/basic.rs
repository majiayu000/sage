//! Basic fallback chain tests

use super::super::manager::FallbackChain;
use super::super::types::{FallbackReason, ModelConfig};

#[tokio::test]
async fn test_fallback_chain_creation() {
    let chain = FallbackChain::new();
    assert!(chain.is_empty().await);
}

#[tokio::test]
async fn test_add_model() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "provider1"))
        .await;

    assert_eq!(chain.model_count().await, 1);
    assert_eq!(chain.current_model().await, Some("model1".to_string()));
}

#[tokio::test]
async fn test_priority_ordering() {
    let chain = FallbackChain::new();

    chain
        .add_model(ModelConfig::new("low", "p").with_priority(10))
        .await;
    chain
        .add_model(ModelConfig::new("high", "p").with_priority(1))
        .await;
    chain
        .add_model(ModelConfig::new("medium", "p").with_priority(5))
        .await;

    let models = chain.list_models().await;
    assert_eq!(models, vec!["high", "medium", "low"]);
}

#[tokio::test]
async fn test_next_available_no_models() {
    let chain = FallbackChain::new();
    let model = chain.next_available(None).await;
    assert_eq!(model, None);
}

#[tokio::test]
async fn test_next_available_all_unhealthy() {
    let chain = FallbackChain::new();
    let mut config = ModelConfig::new("model1", "p");
    config.healthy = false;
    chain.add_model(config).await;

    let model = chain.next_available(None).await;
    assert_eq!(model, None);
}

#[tokio::test]
async fn test_next_available_all_too_small_context() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("small1", "p").with_max_context(1000))
        .await;
    chain
        .add_model(ModelConfig::new("small2", "p").with_max_context(2000))
        .await;

    // Request requiring 10000 context
    let model = chain.next_available(Some(10000)).await;
    assert_eq!(model, None);
}

#[tokio::test]
async fn test_context_size_filtering() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("small", "p").with_max_context(1000))
        .await;
    chain
        .add_model(ModelConfig::new("large", "p").with_max_context(100000))
        .await;

    // Request too large for first model
    let model = chain.next_available(Some(50000)).await;
    assert_eq!(model, Some("large".to_string()));
}

#[tokio::test]
async fn test_default_fallback_chain() {
    let chain = FallbackChain::default();
    assert!(chain.is_empty().await);
}

#[tokio::test]
async fn test_current_model_empty_chain() {
    let chain = FallbackChain::new();
    assert_eq!(chain.current_model().await, None);
}

#[tokio::test]
async fn test_list_models_empty() {
    let chain = FallbackChain::new();
    let models = chain.list_models().await;
    assert!(models.is_empty());
}

#[tokio::test]
async fn test_record_failure_nonexistent_model() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("model1", "p")).await;

    // Record failure for model that doesn't exist
    let next = chain
        .record_failure("nonexistent", FallbackReason::Timeout)
        .await;
    assert_eq!(next, None);
}

#[tokio::test]
async fn test_record_success_nonexistent_model() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("model1", "p")).await;

    // Record success for model that doesn't exist - should not panic
    chain.record_success("nonexistent").await;

    let stats = chain.get_stats().await;
    assert_eq!(stats[0].total_requests, 0);
}
