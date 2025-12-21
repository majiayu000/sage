//! Telemetry and metrics system
//!
//! Provides comprehensive metrics collection for monitoring agent performance,
//! resource usage, and operational health.

pub mod collector;
pub mod metrics;

pub use collector::{
    MetricsCollector, MetricsSnapshot, SharedMetricsCollector, create_metrics_collector,
};
pub use metrics::{
    Counter, Gauge, Histogram, HistogramData, HistogramTimer, LabeledCounter, Metric, MetricType,
    MetricValue,
};
