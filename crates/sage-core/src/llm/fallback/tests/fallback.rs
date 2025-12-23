//! Fallback behavior tests

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::super::manager::FallbackChain;
use super::super::types::{FallbackReason, ModelConfig};

#[tokio::test]
async fn test_record_failure_triggers_fallback() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "p").with_max_retries(1))
        .await;
    chain.add_model(ModelConfig::new("model2", "p")).await;

    // First failure
    let next = chain
        .record_failure("model1", FallbackReason::RateLimited)
        .await;
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
async fn test_force_fallback_no_next_model() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("only_model", "p")).await;

    let next = chain.force_fallback(FallbackReason::Manual).await;
    assert_eq!(next, None);
}

#[tokio::test]
async fn test_force_fallback_skips_unhealthy() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("model1", "p")).await;

    let mut unhealthy = ModelConfig::new("model2", "p");
    unhealthy.healthy = false;
    chain.add_model(unhealthy).await;

    chain.add_model(ModelConfig::new("model3", "p")).await;

    let next = chain.force_fallback(FallbackReason::Manual).await;
    // Should skip unhealthy model2 and go to model3
    assert_eq!(next, Some("model3".to_string()));
}

#[tokio::test]
async fn test_multiple_failures_before_fallback() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "p").with_max_retries(3))
        .await;
    chain.add_model(ModelConfig::new("model2", "p")).await;

    // First two failures should not trigger fallback
    let next1 = chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;
    assert_eq!(next1, None);

    let next2 = chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;
    assert_eq!(next2, None);

    // Third failure should trigger fallback
    let next3 = chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;
    assert_eq!(next3, Some("model2".to_string()));
}

#[tokio::test]
async fn test_fallback_history() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "p").with_max_retries(0))
        .await;
    chain.add_model(ModelConfig::new("model2", "p")).await;

    chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;

    let history = chain.get_history().await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].from_model, "model1");
}

#[tokio::test]
async fn test_history_max_size() {
    let chain = FallbackChain {
        models: Arc::new(RwLock::new(Vec::new())),
        current_index: Arc::new(RwLock::new(0)),
        history: Arc::new(RwLock::new(Vec::new())),
        max_history: 5,
    };

    chain
        .add_model(ModelConfig::new("m1", "p").with_max_retries(0))
        .await;
    chain.add_model(ModelConfig::new("m2", "p")).await;

    // Generate more than max_history events
    for i in 0..10 {
        chain
            .record_failure("m1", FallbackReason::Error(format!("error {}", i)))
            .await;
    }

    let history = chain.get_history().await;
    assert_eq!(history.len(), 5); // Should be capped at max_history
}

#[tokio::test]
async fn test_cooldown_period() {
    let chain = FallbackChain::new();
    chain
        .add_model(
            ModelConfig::new("model1", "p")
                .with_cooldown(Duration::from_millis(100))
                .with_max_retries(0),
        )
        .await;

    // Trigger failure
    chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;

    // Should be unavailable immediately
    let stats = chain.get_stats().await;
    assert!(!stats[0].available);

    // Wait for cooldown
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should be available again
    let model = chain.next_available(None).await;
    assert_eq!(model, Some("model1".to_string()));
}
