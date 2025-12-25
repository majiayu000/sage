//! Gauge metrics - values that can increase or decrease

use std::sync::atomic::{AtomicI64, Ordering};

use super::types::{Metric, MetricType, MetricValue};

/// Gauge metric (can increase or decrease)
#[derive(Debug)]
pub struct Gauge {
    name: String,
    value: AtomicI64,
    scale: f64,
    description: String,
}

impl Gauge {
    /// Create a new gauge
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicI64::new(0),
            scale: 1000.0, // Store as milliunits for precision
            description: description.into(),
        }
    }

    /// Set the gauge value
    pub fn set(&self, value: f64) {
        let scaled = (value * self.scale) as i64;
        self.value.store(scaled, Ordering::Relaxed);
    }

    /// Increment the gauge
    pub fn inc(&self) {
        self.inc_by(1.0);
    }

    /// Increment by a specific amount
    pub fn inc_by(&self, n: f64) {
        let scaled = (n * self.scale) as i64;
        self.value.fetch_add(scaled, Ordering::Relaxed);
    }

    /// Decrement the gauge
    pub fn dec(&self) {
        self.dec_by(1.0);
    }

    /// Decrement by a specific amount
    pub fn dec_by(&self, n: f64) {
        let scaled = (n * self.scale) as i64;
        self.value.fetch_sub(scaled, Ordering::Relaxed);
    }

    /// Get current value
    pub fn get(&self) -> f64 {
        self.value.load(Ordering::Relaxed) as f64 / self.scale
    }

    /// Get description
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Metric for Gauge {
    fn name(&self) -> &str {
        &self.name
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Gauge
    }

    fn value(&self) -> MetricValue {
        MetricValue::Gauge(self.get())
    }

    fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}
