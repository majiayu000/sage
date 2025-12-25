//! Core types for cost tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        let model_stats = self.by_model.entry(record.model_id.clone()).or_default();
        model_stats.input_tokens += record.input_tokens;
        model_stats.output_tokens += record.output_tokens;
        model_stats.cost += record.cost;
        model_stats.call_count += 1;

        // Update provider stats
        let provider_stats = self.by_provider.entry(record.provider.clone()).or_default();
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
