//! Cost tracking for LLM usage
//!
//! This module tracks token usage and calculates costs across sessions.

use super::pricing::{PricingRegistry, TokenPrice};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Usage record for a single LLM call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Unique identifier
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Model ID
    pub model_id: String,
    /// Provider name
    pub provider: String,
    /// Input tokens
    pub input_tokens: usize,
    /// Output tokens
    pub output_tokens: usize,
    /// Calculated cost (USD)
    pub cost: f64,
    /// Session ID (optional)
    pub session_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl UsageRecord {
    /// Create a new usage record
    pub fn new(
        model_id: impl Into<String>,
        provider: impl Into<String>,
        input_tokens: usize,
        output_tokens: usize,
        cost: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            model_id: model_id.into(),
            provider: provider.into(),
            input_tokens,
            output_tokens,
            cost,
            session_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Aggregated usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    /// Total input tokens
    pub total_input_tokens: usize,
    /// Total output tokens
    pub total_output_tokens: usize,
    /// Total cost (USD)
    pub total_cost: f64,
    /// Number of API calls
    pub call_count: usize,
    /// Breakdown by model
    pub by_model: HashMap<String, ModelStats>,
    /// Breakdown by provider
    pub by_provider: HashMap<String, ProviderStats>,
}

impl UsageStats {
    /// Add a usage record to stats
    pub fn add_record(&mut self, record: &UsageRecord) {
        self.total_input_tokens += record.input_tokens;
        self.total_output_tokens += record.output_tokens;
        self.total_cost += record.cost;
        self.call_count += 1;

        // Update model stats
        let model_stats = self
            .by_model
            .entry(record.model_id.clone())
            .or_default();
        model_stats.input_tokens += record.input_tokens;
        model_stats.output_tokens += record.output_tokens;
        model_stats.cost += record.cost;
        model_stats.call_count += 1;

        // Update provider stats
        let provider_stats = self
            .by_provider
            .entry(record.provider.clone())
            .or_default();
        provider_stats.input_tokens += record.input_tokens;
        provider_stats.output_tokens += record.output_tokens;
        provider_stats.cost += record.cost;
        provider_stats.call_count += 1;
    }

    /// Total tokens (input + output)
    pub fn total_tokens(&self) -> usize {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Format cost as string
    pub fn format_cost(&self) -> String {
        if self.total_cost < 0.01 {
            format!("${:.4}", self.total_cost)
        } else if self.total_cost < 1.0 {
            format!("${:.3}", self.total_cost)
        } else {
            format!("${:.2}", self.total_cost)
        }
    }
}

/// Statistics for a single model
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelStats {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
    pub call_count: usize,
}

/// Statistics for a provider
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderStats {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
    pub call_count: usize,
}

/// Cost tracker for monitoring LLM usage
#[derive(Debug)]
pub struct CostTracker {
    /// Pricing registry
    pricing: PricingRegistry,
    /// Usage records
    records: Arc<RwLock<Vec<UsageRecord>>>,
    /// Current session ID
    session_id: Option<String>,
    /// Cost limit (optional)
    cost_limit: Option<f64>,
    /// Warning threshold (percentage of limit)
    warning_threshold: f64,
}

impl CostTracker {
    /// Create a new cost tracker
    pub fn new() -> Self {
        Self {
            pricing: PricingRegistry::with_defaults(),
            records: Arc::new(RwLock::new(Vec::new())),
            session_id: None,
            cost_limit: None,
            warning_threshold: 0.8,
        }
    }

    /// Create with custom pricing registry
    pub fn with_pricing(pricing: PricingRegistry) -> Self {
        Self {
            pricing,
            records: Arc::new(RwLock::new(Vec::new())),
            session_id: None,
            cost_limit: None,
            warning_threshold: 0.8,
        }
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set cost limit
    pub fn with_cost_limit(mut self, limit: f64) -> Self {
        self.cost_limit = Some(limit);
        self
    }

    /// Set warning threshold (0.0 - 1.0)
    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Track usage for an LLM call
    pub async fn track(
        &self,
        model_id: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> TrackResult {
        // Calculate cost
        let (cost, provider) = if let Some(pricing) = self.pricing.get(model_id) {
            (
                pricing.calculate_cost(input_tokens, output_tokens),
                pricing.provider.clone(),
            )
        } else {
            // Unknown model, use zero cost but still track
            tracing::warn!("Unknown model for cost tracking: {}", model_id);
            (0.0, "unknown".to_string())
        };

        // Create record
        let mut record = UsageRecord::new(model_id, provider, input_tokens, output_tokens, cost);
        if let Some(ref session_id) = self.session_id {
            record = record.with_session(session_id);
        }

        // Store record
        {
            let mut records = self.records.write().await;
            records.push(record.clone());
        }

        // Check limits
        let stats = self.get_stats().await;
        let status = self.check_status(&stats);

        TrackResult {
            record,
            cumulative_cost: stats.total_cost,
            status,
        }
    }

    /// Track with custom price (for models not in registry)
    pub async fn track_with_price(
        &self,
        model_id: &str,
        provider: &str,
        input_tokens: usize,
        output_tokens: usize,
        price: TokenPrice,
    ) -> TrackResult {
        let cost = price.calculate(input_tokens, output_tokens);

        let mut record = UsageRecord::new(model_id, provider, input_tokens, output_tokens, cost);
        if let Some(ref session_id) = self.session_id {
            record = record.with_session(session_id);
        }

        {
            let mut records = self.records.write().await;
            records.push(record.clone());
        }

        let stats = self.get_stats().await;
        let status = self.check_status(&stats);

        TrackResult {
            record,
            cumulative_cost: stats.total_cost,
            status,
        }
    }

    /// Get aggregated statistics
    pub async fn get_stats(&self) -> UsageStats {
        let records = self.records.read().await;
        let mut stats = UsageStats::default();

        for record in records.iter() {
            stats.add_record(record);
        }

        stats
    }

    /// Get statistics for a specific session
    pub async fn get_session_stats(&self, session_id: &str) -> UsageStats {
        let records = self.records.read().await;
        let mut stats = UsageStats::default();

        for record in records.iter() {
            if record.session_id.as_deref() == Some(session_id) {
                stats.add_record(record);
            }
        }

        stats
    }

    /// Get all records
    pub async fn get_records(&self) -> Vec<UsageRecord> {
        self.records.read().await.clone()
    }

    /// Get records for a time range
    pub async fn get_records_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<UsageRecord> {
        let records = self.records.read().await;
        records
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Clear all records
    pub async fn clear(&self) {
        self.records.write().await.clear();
    }

    /// Check cost status against limit
    fn check_status(&self, stats: &UsageStats) -> CostStatus {
        match self.cost_limit {
            None => CostStatus::Ok,
            Some(limit) => {
                if stats.total_cost >= limit {
                    CostStatus::LimitExceeded {
                        limit,
                        current: stats.total_cost,
                    }
                } else if stats.total_cost >= limit * self.warning_threshold {
                    CostStatus::Warning {
                        limit,
                        current: stats.total_cost,
                        threshold: self.warning_threshold,
                    }
                } else {
                    CostStatus::Ok
                }
            }
        }
    }

    /// Get pricing registry
    pub fn pricing(&self) -> &PricingRegistry {
        &self.pricing
    }

    /// Estimate cost for a request
    pub fn estimate_cost(&self, model_id: &str, input_tokens: usize, output_tokens: usize) -> f64 {
        self.pricing
            .calculate_cost(model_id, input_tokens, output_tokens)
            .unwrap_or(0.0)
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of tracking usage
#[derive(Debug, Clone)]
pub struct TrackResult {
    /// The usage record created
    pub record: UsageRecord,
    /// Cumulative cost so far
    pub cumulative_cost: f64,
    /// Cost status
    pub status: CostStatus,
}

/// Cost limit status
#[derive(Debug, Clone, PartialEq)]
pub enum CostStatus {
    /// Within limits
    Ok,
    /// Approaching limit
    Warning {
        limit: f64,
        current: f64,
        threshold: f64,
    },
    /// Limit exceeded
    LimitExceeded { limit: f64, current: f64 },
}

impl CostStatus {
    /// Check if status indicates a problem
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::Warning { .. })
    }

    /// Check if limit is exceeded
    pub fn is_exceeded(&self) -> bool {
        matches!(self, Self::LimitExceeded { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        tracker.track("claude-3-5-sonnet-20241022", 2000, 1000).await;

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
}
