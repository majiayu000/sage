//! Telemetry and metrics system
//!
//! Provides comprehensive metrics collection for monitoring agent performance,
//! resource usage, and operational health.

pub mod collector;
pub mod metrics;
pub mod tool_usage;

pub use collector::{
    MetricsCollector, MetricsSnapshot, SharedMetricsCollector, create_metrics_collector,
    global_metrics,
};
pub use metrics::{
    Counter, Gauge, Histogram, HistogramData, HistogramTimer, LabeledCounter, Metric, MetricType,
    MetricValue,
};
pub use tool_usage::{
    TelemetryCollector, TelemetrySummary, ToolStats, ToolUsageEvent, global_telemetry,
};
