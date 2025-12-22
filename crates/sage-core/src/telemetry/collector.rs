//! Metrics collector for aggregating and exporting metrics

use super::metrics::{Counter, Gauge, Histogram, HistogramData, Metric};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metrics collector for agent monitoring
#[derive(Debug)]
pub struct MetricsCollector {
    // LLM metrics
    pub llm_requests: Counter,
    pub llm_tokens_input: Counter,
    pub llm_tokens_output: Counter,
    pub llm_latency: Histogram,
    pub llm_errors: Counter,

    // Tool metrics
    pub tool_calls: Counter,
    pub tool_success: Counter,
    pub tool_errors: Counter,
    pub tool_latency: Histogram,

    // Session metrics
    pub active_sessions: Gauge,
    pub total_sessions: Counter,
    pub session_duration: Histogram,

    // Cache metrics
    pub cache_hits: Counter,
    pub cache_misses: Counter,
    pub cache_size: Gauge,

    // Resource metrics
    pub memory_usage: Gauge,
    pub context_tokens: Gauge,

    // Custom metrics
    custom_counters: Arc<RwLock<HashMap<String, Counter>>>,
    custom_gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    custom_histograms: Arc<RwLock<HashMap<String, Histogram>>>,

    // Collection start time
    started_at: DateTime<Utc>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            // LLM metrics
            llm_requests: Counter::new("llm_requests_total", "Total LLM API requests"),
            llm_tokens_input: Counter::new("llm_tokens_input_total", "Total input tokens"),
            llm_tokens_output: Counter::new("llm_tokens_output_total", "Total output tokens"),
            llm_latency: Histogram::new("llm_request_duration_seconds", "LLM request latency"),
            llm_errors: Counter::new("llm_errors_total", "Total LLM errors"),

            // Tool metrics
            tool_calls: Counter::new("tool_calls_total", "Total tool calls"),
            tool_success: Counter::new("tool_success_total", "Successful tool calls"),
            tool_errors: Counter::new("tool_errors_total", "Failed tool calls"),
            tool_latency: Histogram::new("tool_duration_seconds", "Tool execution latency"),

            // Session metrics
            active_sessions: Gauge::new("sessions_active", "Currently active sessions"),
            total_sessions: Counter::new("sessions_total", "Total sessions created"),
            session_duration: Histogram::with_buckets(
                "session_duration_seconds",
                "Session duration",
                vec![60.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0],
            ),

            // Cache metrics
            cache_hits: Counter::new("cache_hits_total", "Cache hits"),
            cache_misses: Counter::new("cache_misses_total", "Cache misses"),
            cache_size: Gauge::new("cache_size_bytes", "Current cache size"),

            // Resource metrics
            memory_usage: Gauge::new("memory_usage_bytes", "Memory usage"),
            context_tokens: Gauge::new("context_tokens_current", "Current context size"),

            // Custom metrics
            custom_counters: Arc::new(RwLock::new(HashMap::new())),
            custom_gauges: Arc::new(RwLock::new(HashMap::new())),
            custom_histograms: Arc::new(RwLock::new(HashMap::new())),

            started_at: Utc::now(),
        }
    }

    /// Record an LLM request
    pub async fn record_llm_request(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        latency_secs: f64,
        success: bool,
    ) {
        self.llm_requests.inc();
        self.llm_tokens_input.inc_by(input_tokens);
        self.llm_tokens_output.inc_by(output_tokens);
        self.llm_latency.observe(latency_secs);

        if !success {
            self.llm_errors.inc();
        }
    }

    /// Record a tool call
    pub async fn record_tool_call(&self, latency_secs: f64, success: bool) {
        self.tool_calls.inc();
        self.tool_latency.observe(latency_secs);

        if success {
            self.tool_success.inc();
        } else {
            self.tool_errors.inc();
        }
    }

    /// Record session start
    pub fn record_session_start(&self) {
        self.total_sessions.inc();
        self.active_sessions.inc();
    }

    /// Record session end
    pub async fn record_session_end(&self, duration_secs: f64) {
        self.active_sessions.dec();
        self.session_duration.observe(duration_secs);
    }

    /// Record cache access
    pub fn record_cache_hit(&self) {
        self.cache_hits.inc();
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.inc();
    }

    /// Update cache size
    pub fn set_cache_size(&self, size: f64) {
        self.cache_size.set(size);
    }

    /// Update memory usage
    pub fn set_memory_usage(&self, bytes: f64) {
        self.memory_usage.set(bytes);
    }

    /// Update context token count
    pub fn set_context_tokens(&self, tokens: f64) {
        self.context_tokens.set(tokens);
    }

    /// Register a custom counter
    pub async fn register_counter(&self, name: impl Into<String>, description: impl Into<String>) {
        let name = name.into();
        let mut counters = self.custom_counters.write().await;
        if !counters.contains_key(&name) {
            counters.insert(name.clone(), Counter::new(&name, description));
        }
    }

    /// Increment a custom counter
    pub async fn inc_counter(&self, name: &str) {
        if let Some(counter) = self.custom_counters.read().await.get(name) {
            counter.inc();
        }
    }

    /// Increment a custom counter by amount
    pub async fn inc_counter_by(&self, name: &str, n: u64) {
        if let Some(counter) = self.custom_counters.read().await.get(name) {
            counter.inc_by(n);
        }
    }

    /// Register a custom gauge
    pub async fn register_gauge(&self, name: impl Into<String>, description: impl Into<String>) {
        let name = name.into();
        let mut gauges = self.custom_gauges.write().await;
        if !gauges.contains_key(&name) {
            gauges.insert(name.clone(), Gauge::new(&name, description));
        }
    }

    /// Set a custom gauge value
    pub async fn set_gauge(&self, name: &str, value: f64) {
        if let Some(gauge) = self.custom_gauges.read().await.get(name) {
            gauge.set(value);
        }
    }

    /// Register a custom histogram
    pub async fn register_histogram(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) {
        let name = name.into();
        let mut histograms = self.custom_histograms.write().await;
        if !histograms.contains_key(&name) {
            histograms.insert(name.clone(), Histogram::new(&name, description));
        }
    }

    /// Observe a value in a custom histogram
    pub async fn observe_histogram(&self, name: &str, value: f64) {
        if let Some(histogram) = self.custom_histograms.read().await.get(name) {
            histogram.observe(value);
        }
    }

    /// Get a snapshot of all metrics
    pub async fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: Utc::now(),
            uptime_seconds: (Utc::now() - self.started_at).num_seconds() as u64,

            // LLM metrics
            llm_requests: self.llm_requests.get(),
            llm_tokens_input: self.llm_tokens_input.get(),
            llm_tokens_output: self.llm_tokens_output.get(),
            llm_latency: self.llm_latency.get_data(),
            llm_errors: self.llm_errors.get(),

            // Tool metrics
            tool_calls: self.tool_calls.get(),
            tool_success: self.tool_success.get(),
            tool_errors: self.tool_errors.get(),
            tool_latency: self.tool_latency.get_data(),

            // Session metrics
            active_sessions: self.active_sessions.get() as u64,
            total_sessions: self.total_sessions.get(),
            session_duration: self.session_duration.get_data(),

            // Cache metrics
            cache_hits: self.cache_hits.get(),
            cache_misses: self.cache_misses.get(),
            cache_size: self.cache_size.get(),

            // Resource metrics
            memory_usage: self.memory_usage.get(),
            context_tokens: self.context_tokens.get(),

            // Derived metrics
            cache_hit_rate: self.cache_hit_rate(),
            tool_success_rate: self.tool_success_rate(),
            llm_error_rate: self.llm_error_rate(),
        }
    }

    /// Calculate cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.get();
        let misses = self.cache_misses.get();
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Calculate tool success rate
    pub fn tool_success_rate(&self) -> f64 {
        let total = self.tool_calls.get();
        if total == 0 {
            1.0
        } else {
            self.tool_success.get() as f64 / total as f64
        }
    }

    /// Calculate LLM error rate
    pub fn llm_error_rate(&self) -> f64 {
        let total = self.llm_requests.get();
        if total == 0 {
            0.0
        } else {
            self.llm_errors.get() as f64 / total as f64
        }
    }

    /// Get total tokens used
    pub fn total_tokens(&self) -> u64 {
        self.llm_tokens_input.get() + self.llm_tokens_output.get()
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.llm_requests.reset();
        self.llm_tokens_input.reset();
        self.llm_tokens_output.reset();
        self.llm_latency.reset();
        self.llm_errors.reset();
        self.tool_calls.reset();
        self.tool_success.reset();
        self.tool_errors.reset();
        self.tool_latency.reset();
        self.total_sessions.reset();
        self.active_sessions.reset();
        self.session_duration.reset();
        self.cache_hits.reset();
        self.cache_misses.reset();
        self.cache_size.reset();
        self.memory_usage.reset();
        self.context_tokens.reset();

        // Reset custom metrics
        for counter in self.custom_counters.read().await.values() {
            counter.reset();
        }
        for gauge in self.custom_gauges.read().await.values() {
            gauge.reset();
        }
        for histogram in self.custom_histograms.read().await.values() {
            histogram.reset();
        }
    }

    /// Format metrics as human-readable summary
    pub async fn summary(&self) -> String {
        let snapshot = self.snapshot().await;

        format!(
            "Metrics Summary (uptime: {}s)\n\
             LLM: {} requests, {} tokens ({} in / {} out), {:.1}% error rate\n\
             Tools: {} calls, {:.1}% success rate\n\
             Sessions: {} active, {} total\n\
             Cache: {:.1}% hit rate ({} hits / {} misses)",
            snapshot.uptime_seconds,
            snapshot.llm_requests,
            snapshot.llm_tokens_input + snapshot.llm_tokens_output,
            snapshot.llm_tokens_input,
            snapshot.llm_tokens_output,
            snapshot.llm_error_rate * 100.0,
            snapshot.tool_calls,
            snapshot.tool_success_rate * 100.0,
            snapshot.active_sessions,
            snapshot.total_sessions,
            snapshot.cache_hit_rate * 100.0,
            snapshot.cache_hits,
            snapshot.cache_misses,
        )
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.llm_requests.get(), 0);
        assert_eq!(collector.tool_calls.get(), 0);
    }

    #[tokio::test]
    async fn test_record_llm_request() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;

        assert_eq!(collector.llm_requests.get(), 1);
        assert_eq!(collector.llm_tokens_input.get(), 100);
        assert_eq!(collector.llm_tokens_output.get(), 50);
        assert_eq!(collector.llm_errors.get(), 0);
    }

    #[tokio::test]
    async fn test_record_llm_error() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 0, 1.0, false).await;

        assert_eq!(collector.llm_requests.get(), 1);
        assert_eq!(collector.llm_errors.get(), 1);
    }

    #[tokio::test]
    async fn test_record_tool_call() {
        let collector = MetricsCollector::new();

        collector.record_tool_call(0.1, true).await;
        collector.record_tool_call(0.2, false).await;

        assert_eq!(collector.tool_calls.get(), 2);
        assert_eq!(collector.tool_success.get(), 1);
        assert_eq!(collector.tool_errors.get(), 1);
    }

    #[tokio::test]
    async fn test_session_tracking() {
        let collector = MetricsCollector::new();

        collector.record_session_start();
        collector.record_session_start();
        assert_eq!(collector.active_sessions.get() as u64, 2);
        assert_eq!(collector.total_sessions.get(), 2);

        collector.record_session_end(300.0).await;
        assert_eq!(collector.active_sessions.get() as u64, 1);
    }

    #[tokio::test]
    async fn test_cache_metrics() {
        let collector = MetricsCollector::new();

        collector.record_cache_hit();
        collector.record_cache_hit();
        collector.record_cache_miss();

        assert_eq!(collector.cache_hits.get(), 2);
        assert_eq!(collector.cache_misses.get(), 1);
        assert!((collector.cache_hit_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_custom_counter() {
        let collector = MetricsCollector::new();

        collector
            .register_counter("custom_events", "Custom events")
            .await;
        collector.inc_counter("custom_events").await;
        collector.inc_counter_by("custom_events", 5).await;

        let counters = collector.custom_counters.read().await;
        assert_eq!(counters.get("custom_events").unwrap().get(), 6);
    }

    #[tokio::test]
    async fn test_custom_gauge() {
        let collector = MetricsCollector::new();

        collector.register_gauge("queue_size", "Queue size").await;
        collector.set_gauge("queue_size", 42.0).await;

        let gauges = collector.custom_gauges.read().await;
        assert!((gauges.get("queue_size").unwrap().get() - 42.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_custom_histogram() {
        let collector = MetricsCollector::new();

        collector
            .register_histogram("custom_duration", "Custom duration")
            .await;
        collector.observe_histogram("custom_duration", 0.5).await;

        let histograms = collector.custom_histograms.read().await;
        let data = histograms.get("custom_duration").unwrap().get_data();
        assert_eq!(data.count, 1);
    }

    #[tokio::test]
    async fn test_snapshot() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_tool_call(0.1, true).await;

        let snapshot = collector.snapshot().await;

        assert_eq!(snapshot.llm_requests, 1);
        assert_eq!(snapshot.total_tokens(), 150);
        assert_eq!(snapshot.tool_calls, 1);
    }

    #[tokio::test]
    async fn test_success_rates() {
        let collector = MetricsCollector::new();

        // Tool success rate
        collector.record_tool_call(0.1, true).await;
        collector.record_tool_call(0.1, true).await;
        collector.record_tool_call(0.1, false).await;

        assert!((collector.tool_success_rate() - 0.666).abs() < 0.01);

        // LLM error rate
        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_llm_request(100, 0, 1.0, false).await;

        assert!((collector.llm_error_rate() - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_reset() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_tool_call(0.1, true).await;

        collector.reset().await;

        assert_eq!(collector.llm_requests.get(), 0);
        assert_eq!(collector.tool_calls.get(), 0);
    }

    #[tokio::test]
    async fn test_summary() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(1000, 500, 0.5, true).await;
        collector.record_tool_call(0.1, true).await;

        let summary = collector.summary().await;

        assert!(summary.contains("LLM:"));
        assert!(summary.contains("Tools:"));
        assert!(summary.contains("1500 tokens"));
    }

    #[tokio::test]
    async fn test_total_tokens() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_llm_request(200, 100, 0.3, true).await;

        assert_eq!(collector.total_tokens(), 450);
    }

    #[test]
    fn test_shared_collector() {
        let collector = create_metrics_collector();
        collector.llm_requests.inc();
        assert_eq!(collector.llm_requests.get(), 1);
    }
}
