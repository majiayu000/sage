//! Counter metrics - monotonically increasing values

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// Use parking_lot::RwLock for synchronous, non-blocking access
use parking_lot::RwLock;

use super::types::{Metric, MetricType, MetricValue};

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

/// Labeled counter for tracking multiple series
///
/// Uses parking_lot::RwLock for synchronous, non-blocking access.
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
    pub fn inc(&self, labels: [impl Into<String>; N]) {
        self.inc_by(labels, 1);
    }

    /// Increment counter by amount for given labels
    pub fn inc_by(&self, labels: [impl Into<String>; N], n: u64) {
        let labels: [String; N] = labels.map(|s| s.into());
        let mut counters = self.counters.write();

        if let Some(counter) = counters.get(&labels) {
            counter.fetch_add(n, Ordering::Relaxed);
        } else {
            let counter = AtomicU64::new(n);
            counters.insert(labels, counter);
        }
    }

    /// Get counter value for labels
    pub fn get(&self, labels: &[String; N]) -> u64 {
        let counters = self.counters.read();
        counters
            .get(labels)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get all values
    pub fn get_all(&self) -> Vec<([String; N], u64)> {
        let counters = self.counters.read();
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
