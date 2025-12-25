//! Types and type aliases for metrics collection

use super::super::metrics::HistogramData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::MetricsCollector;

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// Seconds since collector started
    pub uptime_seconds: u64,

    // LLM metrics
    pub llm_requests: u64,
    pub llm_tokens_input: u64,
    pub llm_tokens_output: u64,
    pub llm_latency: HistogramData,
    pub llm_errors: u64,

    // Tool metrics
    pub tool_calls: u64,
    pub tool_success: u64,
    pub tool_errors: u64,
    pub tool_latency: HistogramData,

    // Session metrics
    pub active_sessions: u64,
    pub total_sessions: u64,
    pub session_duration: HistogramData,

    // Cache metrics
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size: f64,

    // Resource metrics
    pub memory_usage: f64,
    pub context_tokens: f64,

    // Derived metrics
    pub cache_hit_rate: f64,
    pub tool_success_rate: f64,
    pub llm_error_rate: f64,
}

impl MetricsSnapshot {
    /// Total tokens used
    pub fn total_tokens(&self) -> u64 {
        self.llm_tokens_input + self.llm_tokens_output
    }

    /// Average LLM latency
    pub fn avg_llm_latency(&self) -> f64 {
        self.llm_latency.mean()
    }

    /// Average tool latency
    pub fn avg_tool_latency(&self) -> f64 {
        self.tool_latency.mean()
    }
}

/// Thread-safe shared metrics collector
pub type SharedMetricsCollector = Arc<MetricsCollector>;

/// Create a shared metrics collector
pub fn create_metrics_collector() -> SharedMetricsCollector {
    Arc::new(MetricsCollector::new())
}

/// Global metrics collector for the application
static GLOBAL_METRICS: once_cell::sync::Lazy<MetricsCollector> =
    once_cell::sync::Lazy::new(MetricsCollector::new);

/// Get the global metrics collector
pub fn global_metrics() -> &'static MetricsCollector {
    &GLOBAL_METRICS
}
