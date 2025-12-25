//! Tests for metric types

use std::time::Duration;

use super::counter::{Counter, LabeledCounter};
use super::gauge::Gauge;
use super::histogram::Histogram;
use super::types::{HistogramData, Metric, MetricType, MetricValue};

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

#[test]
fn test_histogram_basic() {
    let histogram = Histogram::new("request_duration", "Request duration in seconds");

    histogram.observe(0.1);
    histogram.observe(0.2);
    histogram.observe(0.3);

    let data = histogram.get_data();
    assert_eq!(data.count, 3);
    assert!((data.sum - 0.6).abs() < 0.001);
    assert!((data.min - 0.1).abs() < 0.001);
    assert!((data.max - 0.3).abs() < 0.001);
}

#[test]
fn test_histogram_buckets() {
    let histogram = Histogram::with_buckets("test", "Test", vec![0.1, 0.5, 1.0]);

    histogram.observe(0.05);
    histogram.observe(0.3);
    histogram.observe(0.8);

    let data = histogram.get_data();
    assert_eq!(data.buckets[0], (0.1, 1)); // 0.05 <= 0.1
    assert_eq!(data.buckets[1], (0.5, 2)); // 0.05, 0.3 <= 0.5
    assert_eq!(data.buckets[2], (1.0, 3)); // all <= 1.0
}

#[test]
fn test_histogram_timer() {
    let histogram = Histogram::new("test", "Test");
    let timer = histogram.start_timer();

    std::thread::sleep(Duration::from_millis(10));
    timer.stop();

    let data = histogram.get_data();
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

#[test]
fn test_labeled_counter() {
    let counter: LabeledCounter<2> = LabeledCounter::new(
        "http_requests",
        "HTTP requests by method and status",
        ["method", "status"],
    );

    counter.inc(["GET", "200"]);
    counter.inc(["GET", "200"]);
    counter.inc(["POST", "201"]);

    assert_eq!(counter.get(&["GET".to_string(), "200".to_string()]), 2);
    assert_eq!(counter.get(&["POST".to_string(), "201".to_string()]), 1);
}

#[test]
fn test_labeled_counter_get_all() {
    let counter: LabeledCounter<1> = LabeledCounter::new("test", "Test", ["label"]);

    counter.inc(["a"]);
    counter.inc_by(["b"], 5);

    let all = counter.get_all();
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
