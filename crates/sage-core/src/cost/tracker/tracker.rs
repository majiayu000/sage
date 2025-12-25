//! Cost tracker implementation

use super::calculator::check_status;
use super::types::{TrackResult, UsageRecord, UsageStats};
use crate::cost::pricing::{PricingRegistry, TokenPrice};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

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
        let status = check_status(&stats, self.cost_limit, self.warning_threshold);

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
        let status = check_status(&stats, self.cost_limit, self.warning_threshold);

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
