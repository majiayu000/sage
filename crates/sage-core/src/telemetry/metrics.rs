//! Metric types and definitions

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

/// Counter metric (monotonically increasing)
#[derive(Debug)]
pub struct Counter {
    name: String,
    value: AtomicU64,
    description: String,
}

impl Counter {
    /// Create a new counter
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicU64::new(0),
            description: description.into(),
        }
    }

    /// Increment by 1
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment by a specific amount
    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Get current count
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Get description
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Metric for Counter {
    fn name(&self) -> &str {
        &self.name
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Counter
    }

    fn value(&self) -> MetricValue {
        MetricValue::Counter(self.get())
    }

    fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

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
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
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
    pub async fn observe(&self, value: f64) {
        let mut data = self.data.write().await;
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
    pub async fn observe_duration(&self, duration: Duration) {
        self.observe(duration.as_secs_f64()).await;
    }

    /// Time an operation
    pub fn start_timer(&self) -> HistogramTimer<'_> {
        HistogramTimer {
            histogram: self,
            start: Instant::now(),
        }
    }

    /// Get histogram data
    pub async fn get_data(&self) -> HistogramData {
        let data = self.data.read().await;
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
        // This is blocking - for async use get_data()
        let data = futures::executor::block_on(self.data.read());
        MetricValue::Histogram(HistogramData {
            count: data.count,
            sum: data.sum,
            min: if data.count > 0 { data.min } else { 0.0 },
            max: if data.count > 0 { data.max } else { 0.0 },
            buckets: data.buckets.clone(),
        })
    }

    fn reset(&self) {
        let mut data = futures::executor::block_on(self.data.write());
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
    pub async fn stop(self) {
        let duration = self.start.elapsed();
        self.histogram.observe_duration(duration).await;
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Labeled counter for tracking multiple series
#[derive(Debug)]
pub struct LabeledCounter<const N: usize> {
    name: String,
    description: String,
    label_names: [String; N],
    counters: Arc<RwLock<std::collections::HashMap<[String; N], AtomicU64>>>,
}

impl<const N: usize> LabeledCounter<N> {
    /// Create a new labeled counter
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        label_names: [impl Into<String>; N],
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            label_names: label_names.map(|s| s.into()),
            counters: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Increment counter for given labels
    pub async fn inc(&self, labels: [impl Into<String>; N]) {
        self.inc_by(labels, 1).await;
    }

    /// Increment counter by amount for given labels
    pub async fn inc_by(&self, labels: [impl Into<String>; N], n: u64) {
        let labels: [String; N] = labels.map(|s| s.into());
        let mut counters = self.counters.write().await;

        if let Some(counter) = counters.get(&labels) {
            counter.fetch_add(n, Ordering::Relaxed);
        } else {
            let counter = AtomicU64::new(n);
            counters.insert(labels, counter);
        }
    }

    /// Get counter value for labels
    pub async fn get(&self, labels: &[String; N]) -> u64 {
        let counters = self.counters.read().await;
        counters
            .get(labels)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get all values
    pub async fn get_all(&self) -> Vec<([String; N], u64)> {
        let counters = self.counters.read().await;
        counters
            .iter()
            .map(|(labels, counter)| (labels.clone(), counter.load(Ordering::Relaxed)))
            .collect()
    }

    /// Get name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get label names
    pub fn label_names(&self) -> &[String; N] {
        &self.label_names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_basic() {
        let counter = Counter::new("requests_total", "Total requests");

        counter.inc();
        counter.inc();
        counter.inc_by(5);

        assert_eq!(counter.get(), 7);
        assert_eq!(counter.name(), "requests_total");
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new("test", "Test counter");
        counter.inc_by(100);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_gauge_basic() {
        let gauge = Gauge::new("temperature", "Current temperature");

        gauge.set(25.5);
        assert!((gauge.get() - 25.5).abs() < 0.001);

        gauge.inc_by(2.0);
        assert!((gauge.get() - 27.5).abs() < 0.001);

        gauge.dec_by(5.0);
        assert!((gauge.get() - 22.5).abs() < 0.001);
    }

    #[test]
    fn test_gauge_reset() {
        let gauge = Gauge::new("test", "Test gauge");
        gauge.set(100.0);
        gauge.reset();
        assert!((gauge.get()).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_histogram_basic() {
        let histogram = Histogram::new("request_duration", "Request duration in seconds");

        histogram.observe(0.1).await;
        histogram.observe(0.2).await;
        histogram.observe(0.3).await;

        let data = histogram.get_data().await;
        assert_eq!(data.count, 3);
        assert!((data.sum - 0.6).abs() < 0.001);
        assert!((data.min - 0.1).abs() < 0.001);
        assert!((data.max - 0.3).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_histogram_buckets() {
        let histogram = Histogram::with_buckets("test", "Test", vec![0.1, 0.5, 1.0]);

        histogram.observe(0.05).await;
        histogram.observe(0.3).await;
        histogram.observe(0.8).await;

        let data = histogram.get_data().await;
        assert_eq!(data.buckets[0], (0.1, 1));  // 0.05 <= 0.1
        assert_eq!(data.buckets[1], (0.5, 2));  // 0.05, 0.3 <= 0.5
        assert_eq!(data.buckets[2], (1.0, 3));  // all <= 1.0
    }

    #[tokio::test]
    async fn test_histogram_timer() {
        let histogram = Histogram::new("test", "Test");
        let timer = histogram.start_timer();

        tokio::time::sleep(Duration::from_millis(10)).await;
        timer.stop().await;

        let data = histogram.get_data().await;
        assert_eq!(data.count, 1);
        assert!(data.sum >= 0.01); // At least 10ms
    }

    #[test]
    fn test_histogram_data_mean() {
        let data = HistogramData {
            count: 4,
            sum: 10.0,
            min: 1.0,
            max: 4.0,
            buckets: vec![],
        };
        assert!((data.mean() - 2.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_labeled_counter() {
        let counter: LabeledCounter<2> = LabeledCounter::new(
            "http_requests",
            "HTTP requests by method and status",
            ["method", "status"],
        );

        counter.inc(["GET", "200"]).await;
        counter.inc(["GET", "200"]).await;
        counter.inc(["POST", "201"]).await;

        assert_eq!(counter.get(&["GET".to_string(), "200".to_string()]).await, 2);
        assert_eq!(counter.get(&["POST".to_string(), "201".to_string()]).await, 1);
    }

    #[tokio::test]
    async fn test_labeled_counter_get_all() {
        let counter: LabeledCounter<1> = LabeledCounter::new("test", "Test", ["label"]);

        counter.inc(["a"]).await;
        counter.inc_by(["b"], 5).await;

        let all = counter.get_all().await;
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_metric_trait() {
        let counter = Counter::new("test", "Test counter");
        counter.inc_by(42);

        assert_eq!(counter.metric_type(), MetricType::Counter);
        assert!(matches!(counter.value(), MetricValue::Counter(42)));
    }

    #[test]
    fn test_gauge_negative() {
        let gauge = Gauge::new("balance", "Account balance");
        gauge.set(-100.0);
        assert!((gauge.get() - (-100.0)).abs() < 0.001);
    }
}
