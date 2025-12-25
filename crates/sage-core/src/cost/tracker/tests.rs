//! Tests for cost tracking

#![cfg(test)]

use super::types::{CostStatus, UsageRecord, UsageStats};
use super::tracker::CostTracker;
use crate::cost::pricing::TokenPrice;

#[test]
fn test_usage_record_creation() {
    let record = UsageRecord::new("gpt-4o", "openai", 1000, 500, 0.03);

    assert_eq!(record.model_id, "gpt-4o");
    assert_eq!(record.provider, "openai");
    assert_eq!(record.input_tokens, 1000);
    assert_eq!(record.output_tokens, 500);
    assert!(!record.id.is_empty());
}

#[test]
fn test_usage_record_with_session() {
    let record = UsageRecord::new("claude-3-5-sonnet", "anthropic", 500, 200, 0.01)
        .with_session("session-123");

    assert_eq!(record.session_id, Some("session-123".to_string()));
}

#[test]
fn test_usage_stats_aggregation() {
    let mut stats = UsageStats::default();

    let record1 = UsageRecord::new("gpt-4o", "openai", 1000, 500, 0.03);
    let record2 = UsageRecord::new("gpt-4o", "openai", 2000, 1000, 0.06);

    stats.add_record(&record1);
    stats.add_record(&record2);

    assert_eq!(stats.total_input_tokens, 3000);
    assert_eq!(stats.total_output_tokens, 1500);
    assert!((stats.total_cost - 0.09).abs() < 0.001);
    assert_eq!(stats.call_count, 2);
    assert_eq!(stats.total_tokens(), 4500);
}

#[test]
fn test_usage_stats_by_model() {
    let mut stats = UsageStats::default();

    let record1 = UsageRecord::new("gpt-4o", "openai", 1000, 500, 0.03);
    let record2 = UsageRecord::new("claude-3-5-sonnet", "anthropic", 2000, 1000, 0.05);

    stats.add_record(&record1);
    stats.add_record(&record2);

    assert!(stats.by_model.contains_key("gpt-4o"));
    assert!(stats.by_model.contains_key("claude-3-5-sonnet"));
    assert_eq!(stats.by_model.get("gpt-4o").unwrap().input_tokens, 1000);
}

#[test]
fn test_usage_stats_by_provider() {
    let mut stats = UsageStats::default();

    let record1 = UsageRecord::new("gpt-4o", "openai", 1000, 500, 0.03);
    let record2 = UsageRecord::new("gpt-4o-mini", "openai", 500, 200, 0.01);

    stats.add_record(&record1);
    stats.add_record(&record2);

    let openai_stats = stats.by_provider.get("openai").unwrap();
    assert_eq!(openai_stats.call_count, 2);
    assert_eq!(openai_stats.input_tokens, 1500);
}

#[test]
fn test_format_cost() {
    let mut stats = UsageStats::default();

    stats.total_cost = 0.001;
    assert_eq!(stats.format_cost(), "$0.0010");

    stats.total_cost = 0.05;
    assert_eq!(stats.format_cost(), "$0.050");

    stats.total_cost = 1.5;
    assert_eq!(stats.format_cost(), "$1.50");
}

#[tokio::test]
async fn test_cost_tracker_basic() {
    let tracker = CostTracker::new();

    let result = tracker.track("gpt-4o", 1000, 500).await;

    assert_eq!(result.record.model_id, "gpt-4o");
    assert!(result.record.cost > 0.0);
    assert_eq!(result.status, CostStatus::Ok);
}

#[tokio::test]
async fn test_cost_tracker_with_session() {
    let tracker = CostTracker::new().with_session("test-session");

    let result = tracker.track("gpt-4o", 1000, 500).await;

    assert_eq!(result.record.session_id, Some("test-session".to_string()));
}

#[tokio::test]
async fn test_cost_tracker_limit_warning() {
    let tracker = CostTracker::new()
        .with_cost_limit(0.10)
        .with_warning_threshold(0.5);

    // First call should be OK
    let result1 = tracker.track("gpt-4o", 1000, 500).await;
    assert_eq!(result1.status, CostStatus::Ok);

    // Track more to trigger warning (80% of limit)
    tracker.track("gpt-4o", 10000, 5000).await;
    let result2 = tracker.track("gpt-4o", 10000, 5000).await;

    assert!(result2.status.is_warning() || result2.status.is_exceeded());
}

#[tokio::test]
async fn test_cost_tracker_limit_exceeded() {
    let tracker = CostTracker::new().with_cost_limit(0.001);

    // Should exceed immediately with reasonable usage
    let result = tracker.track("gpt-4o", 10000, 5000).await;

    assert!(result.status.is_exceeded());
}

#[tokio::test]
async fn test_cost_tracker_get_stats() {
    let tracker = CostTracker::new();

    tracker.track("gpt-4o", 1000, 500).await;
    tracker
        .track("claude-3-5-sonnet-20241022", 2000, 1000)
        .await;

    let stats = tracker.get_stats().await;

    assert_eq!(stats.call_count, 2);
    assert!(stats.total_cost > 0.0);
}

#[tokio::test]
async fn test_cost_tracker_session_stats() {
    let tracker1 = CostTracker::new().with_session("session-1");
    let tracker2 = CostTracker::new().with_session("session-2");

    tracker1.track("gpt-4o", 1000, 500).await;
    tracker2.track("gpt-4o", 2000, 1000).await;

    let stats1 = tracker1.get_session_stats("session-1").await;
    assert_eq!(stats1.call_count, 1);

    let stats2 = tracker2.get_session_stats("session-2").await;
    assert_eq!(stats2.call_count, 1);
}

#[tokio::test]
async fn test_cost_tracker_unknown_model() {
    let tracker = CostTracker::new();

    let result = tracker.track("unknown-model-xyz", 1000, 500).await;

    assert_eq!(result.record.cost, 0.0);
    assert_eq!(result.record.provider, "unknown");
}

#[tokio::test]
async fn test_cost_tracker_with_custom_price() {
    let tracker = CostTracker::new();
    let custom_price = TokenPrice::new(5.0, 10.0);

    let result = tracker
        .track_with_price("custom-model", "custom", 1_000_000, 1_000_000, custom_price)
        .await;

    assert!((result.record.cost - 15.0).abs() < 0.001);
}

#[tokio::test]
async fn test_cost_tracker_clear() {
    let tracker = CostTracker::new();

    tracker.track("gpt-4o", 1000, 500).await;
    let stats_before = tracker.get_stats().await;
    assert_eq!(stats_before.call_count, 1);

    tracker.clear().await;
    let stats_after = tracker.get_stats().await;
    assert_eq!(stats_after.call_count, 0);
}

#[tokio::test]
async fn test_cost_tracker_estimate() {
    let tracker = CostTracker::new();

    let estimated = tracker.estimate_cost("gpt-4o", 1_000_000, 1_000_000);

    // GPT-4o: $2.50/1M input, $10/1M output = $12.50 total
    assert!((estimated - 12.5).abs() < 0.001);
}

#[test]
fn test_cost_status_checks() {
    assert!(!CostStatus::Ok.is_warning());
    assert!(!CostStatus::Ok.is_exceeded());

    let warning = CostStatus::Warning {
        limit: 1.0,
        current: 0.8,
        threshold: 0.8,
    };
    assert!(warning.is_warning());
    assert!(!warning.is_exceeded());

    let exceeded = CostStatus::LimitExceeded {
        limit: 1.0,
        current: 1.5,
    };
    assert!(!exceeded.is_warning());
    assert!(exceeded.is_exceeded());
}

#[tokio::test]
async fn test_get_records() {
    let tracker = CostTracker::new();

    tracker.track("gpt-4o", 1000, 500).await;
    tracker.track("gpt-4o", 2000, 1000).await;

    let records = tracker.get_records().await;
    assert_eq!(records.len(), 2);
}
