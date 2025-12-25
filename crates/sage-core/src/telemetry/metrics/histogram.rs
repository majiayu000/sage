//! Histogram metrics - distribution tracking

use std::sync::Arc;
use std::time::{Duration, Instant};

// Use parking_lot::RwLock for synchronous, non-blocking access
use parking_lot::RwLock;

use super::types::{HistogramData, Metric, MetricType, MetricValue};

/// Histogram metric (distribution tracking)
#[derive(Debug)]
pub struct Histogram {
    name: String,
    data: Arc<RwLock<HistogramInner>>,
    description: String,
}

#[derive(Debug)]
struct HistogramInner {
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
    buckets: Vec<(f64, u64)>,
}

impl Histogram {
    /// Create a new histogram with default buckets
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::with_buckets(
            name,
            description,
            vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
    }

    /// Create with custom bucket boundaries
    pub fn with_buckets(
        name: impl Into<String>,
        description: impl Into<String>,
        bucket_bounds: Vec<f64>,
    ) -> Self {
        let buckets = bucket_bounds.into_iter().map(|b| (b, 0u64)).collect();

        Self {
            name: name.into(),
            data: Arc::new(RwLock::new(HistogramInner {
                count: 0,
                sum: 0.0,
                min: f64::MAX,
                max: f64::MIN,
                buckets,
            })),
            description: description.into(),
        }
    }

    /// Observe a value
    ///
    /// This method is synchronous for compatibility with the Metric trait.
    /// Uses parking_lot::RwLock for fast, non-blocking access.
    pub fn observe(&self, value: f64) {
        let mut data = self.data.write();
        data.count += 1;
        data.sum += value;
        data.min = data.min.min(value);
        data.max = data.max.max(value);

        // Update bucket counts
        for (bound, count) in &mut data.buckets {
            if value <= *bound {
                *count += 1;
            }
        }
    }

    /// Observe a duration
    pub fn observe_duration(&self, duration: Duration) {
        self.observe(duration.as_secs_f64());
    }

    /// Time an operation
    pub fn start_timer(&self) -> HistogramTimer<'_> {
        HistogramTimer {
            histogram: self,
            start: Instant::now(),
        }
    }

    /// Get histogram data
    ///
    /// This method is synchronous for compatibility with the Metric trait.
    pub fn get_data(&self) -> HistogramData {
        let data = self.data.read();
        HistogramData {
            count: data.count,
            sum: data.sum,
            min: if data.count > 0 { data.min } else { 0.0 },
            max: if data.count > 0 { data.max } else { 0.0 },
            buckets: data.buckets.clone(),
        }
    }

    /// Get description
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Metric for Histogram {
    fn name(&self) -> &str {
        &self.name
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Histogram
    }

    fn value(&self) -> MetricValue {
        // Uses parking_lot::RwLock for fast, non-blocking read access
        MetricValue::Histogram(self.get_data())
    }

    fn reset(&self) {
        // Uses parking_lot::RwLock for fast, non-blocking write access
        let mut data = self.data.write();
        data.count = 0;
        data.sum = 0.0;
        data.min = f64::MAX;
        data.max = f64::MIN;
        for (_, count) in &mut data.buckets {
            *count = 0;
        }
    }
}

/// Timer for measuring operation duration
pub struct HistogramTimer<'a> {
    histogram: &'a Histogram,
    start: Instant,
}

impl<'a> HistogramTimer<'a> {
    /// Stop the timer and record the duration
    pub fn stop(self) {
        let duration = self.start.elapsed();
        self.histogram.observe_duration(duration);
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}
