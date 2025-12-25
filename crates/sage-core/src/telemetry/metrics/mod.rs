//! Metric types and definitions
//!
//! This module provides a comprehensive metrics system with support for:
//! - Counters: Monotonically increasing values
//! - Gauges: Values that can increase or decrease
//! - Histograms: Distribution tracking with buckets
//! - Labeled metrics: Multi-dimensional metric tracking

mod counter;
mod gauge;
mod histogram;
mod types;

#[cfg(test)]
mod tests;

// Re-export all public items
pub use counter::{Counter, LabeledCounter};
pub use gauge::Gauge;
pub use histogram::{Histogram, HistogramTimer};
pub use types::{HistogramData, Metric, MetricType, MetricValue};
