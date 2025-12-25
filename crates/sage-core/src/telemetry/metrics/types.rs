//! Metric types and trait definitions

use serde::{Deserialize, Serialize};

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter (monotonically increasing)
    Counter,
    /// Gauge (can go up or down)
    Gauge,
    /// Histogram (distribution of values)
    Histogram,
}

/// Metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// Integer counter value
    Counter(u64),
    /// Floating point gauge value
    Gauge(f64),
    /// Histogram data
    Histogram(HistogramData),
}

/// Histogram data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistogramData {
    /// Number of observations
    pub count: u64,
    /// Sum of all observations
    pub sum: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Bucket counts (for percentile calculation)
    pub buckets: Vec<(f64, u64)>,
}

impl HistogramData {
    /// Calculate mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

/// Generic metric trait
pub trait Metric: Send + Sync {
    /// Get metric name
    fn name(&self) -> &str;

    /// Get metric type
    fn metric_type(&self) -> MetricType;

    /// Get current value
    fn value(&self) -> MetricValue;

    /// Reset the metric
    fn reset(&self);
}
