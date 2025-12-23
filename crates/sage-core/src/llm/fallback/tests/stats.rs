//! Statistics and health tests

use super::super::manager::FallbackChain;
use super::super::types::{FallbackReason, ModelConfig};

#[tokio::test]
async fn test_record_success() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "provider1"))
        .await;

    chain.record_success("model1").await;

    let stats = chain.get_stats().await;
    assert_eq!(stats[0].total_requests, 1);
    assert_eq!(stats[0].successful_requests, 1);
}

#[tokio::test]
async fn test_model_stats() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "provider1"))
        .await;

    chain.record_success("model1").await;
    chain.record_success("model1").await;
    chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;

    let stats = chain.get_stats().await;
    assert_eq!(stats[0].total_requests, 3);
    assert_eq!(stats[0].successful_requests, 2);
    assert!((stats[0].success_rate - 0.666).abs() < 0.01);
}

#[tokio::test]
async fn test_success_resets_failure_count() {
    let chain = FallbackChain::new();
    chain
        .add_model(ModelConfig::new("model1", "p").with_max_retries(2))
        .await;

    // Fail once
    chain
        .record_failure("model1", FallbackReason::Timeout)
        .await;

    // Then succeed
    chain.record_success("model1").await;

    // Check that failure count is reset
    let stats = chain.get_stats().await;
    assert_eq!(stats[0].failure_count, 0);
}

#[tokio::test]
async fn test_success_rate_calculation() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("model1", "p")).await;

    // 7 successes, 3 failures = 0.7 success rate
    for _ in 0..7 {
        chain.record_success("model1").await;
    }
    for _ in 0..3 {
        chain
            .record_failure("model1", FallbackReason::Timeout)
            .await;
    }

    let stats = chain.get_stats().await;
    assert_eq!(stats[0].total_requests, 10);
    assert_eq!(stats[0].successful_requests, 7);
    assert!((stats[0].success_rate - 0.7).abs() < 0.01);
}

#[tokio::test]
async fn test_get_stats_empty() {
    let chain = FallbackChain::new();
    let stats = chain.get_stats().await;
    assert!(stats.is_empty());
}

#[tokio::test]
async fn test_get_history_empty() {
    let chain = FallbackChain::new();
    let history = chain.get_history().await;
    assert!(history.is_empty());
}

#[tokio::test]
async fn test_reset_model() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("model1", "p")).await;

    chain
        .record_failure("model1", FallbackReason::Error("test".into()))
        .await;
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
async fn test_reset_model_nonexistent() {
    let chain = FallbackChain::new();
    chain.add_model(ModelConfig::new("model1", "p")).await;

    // Reset nonexistent model - should not panic
    chain.reset_model("nonexistent").await;
}
