//! Metrics collector for aggregating and exporting metrics

mod collector;
mod tests;
mod types;

pub use collector::MetricsCollector;
pub use types::{
    MetricsSnapshot, SharedMetricsCollector, create_metrics_collector, global_metrics,
};
