//! Thread-safe violation storage following Claude Code patterns.

use super::types::{Violation, ViolationSeverity, ViolationType};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Thread-safe violation store
pub type SharedViolationStore = Arc<ViolationStore>;

/// Store for recording and querying violations
#[derive(Debug)]
pub struct ViolationStore {
    /// All recorded violations
    violations: RwLock<Vec<Violation>>,
    /// Count by type for quick lookup
    counts: RwLock<HashMap<ViolationType, usize>>,
    /// Maximum violations to store (prevents memory exhaustion)
    max_violations: usize,
}

impl Default for ViolationStore {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl ViolationStore {
    /// Create a new violation store with max capacity
    pub fn new(max_violations: usize) -> Self {
        Self {
            violations: RwLock::new(Vec::new()),
            counts: RwLock::new(HashMap::new()),
            max_violations,
        }
    }

    /// Create a shared (Arc-wrapped) store
    pub fn shared(max_violations: usize) -> SharedViolationStore {
        Arc::new(Self::new(max_violations))
    }

    /// Record a new violation
    pub fn record(&self, violation: Violation) {
        let vtype = violation.violation_type;

        let mut violations = self.violations.write();

        // Enforce max capacity by removing oldest
        if violations.len() >= self.max_violations {
            if let Some(removed) = violations.first() {
                let removed_type = removed.violation_type;
                let mut counts = self.counts.write();
                if let Some(count) = counts.get_mut(&removed_type) {
                    *count = count.saturating_sub(1);
                }
            }
            violations.remove(0);
        }

        violations.push(violation);
        drop(violations);

        // Update count
        let mut counts = self.counts.write();
        *counts.entry(vtype).or_insert(0) += 1;
    }

    /// Get all violations
    pub fn get_all(&self) -> Vec<Violation> {
        self.violations.read().clone()
    }

    /// Get violations of a specific type
    pub fn get_by_type(&self, vtype: ViolationType) -> Vec<Violation> {
        self.violations
            .read()
            .iter()
            .filter(|v| v.violation_type == vtype)
            .cloned()
            .collect()
    }

    /// Get violations at or above a severity level
    pub fn get_by_severity(&self, min_severity: ViolationSeverity) -> Vec<Violation> {
        self.violations
            .read()
            .iter()
            .filter(|v| v.severity >= min_severity)
            .cloned()
            .collect()
    }

    /// Get blocked violations only
    pub fn get_blocked(&self) -> Vec<Violation> {
        self.violations
            .read()
            .iter()
            .filter(|v| v.blocked)
            .cloned()
            .collect()
    }

    /// Get count by type
    pub fn count_by_type(&self, vtype: ViolationType) -> usize {
        *self.counts.read().get(&vtype).unwrap_or(&0)
    }

    /// Get total violation count
    pub fn total_count(&self) -> usize {
        self.violations.read().len()
    }

    /// Get count of blocked violations
    pub fn blocked_count(&self) -> usize {
        self.violations.read().iter().filter(|v| v.blocked).count()
    }

    /// Check if any critical violations occurred
    pub fn has_critical(&self) -> bool {
        self.violations
            .read()
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical)
    }

    /// Clear all violations
    pub fn clear(&self) {
        self.violations.write().clear();
        self.counts.write().clear();
    }

    /// Get the most recent violations (up to n)
    pub fn get_recent(&self, n: usize) -> Vec<Violation> {
        let violations = self.violations.read();
        let start = violations.len().saturating_sub(n);
        violations[start..].to_vec()
    }

    /// Get a summary of violations
    pub fn summary(&self) -> ViolationSummary {
        let violations = self.violations.read();
        let counts = self.counts.read();

        ViolationSummary {
            total: violations.len(),
            blocked: violations.iter().filter(|v| v.blocked).count(),
            by_type: counts.clone(),
            by_severity: violations.iter().fold(HashMap::new(), |mut acc, v| {
                *acc.entry(v.severity).or_insert(0) += 1;
                acc
            }),
            has_critical: violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Critical),
        }
    }
}

/// Summary of violations
#[derive(Debug, Clone)]
pub struct ViolationSummary {
    /// Total number of violations
    pub total: usize,
    /// Number of blocked violations
    pub blocked: usize,
    /// Count by type
    pub by_type: HashMap<ViolationType, usize>,
    /// Count by severity
    pub by_severity: HashMap<ViolationSeverity, usize>,
    /// Whether any critical violations occurred
    pub has_critical: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_record_and_get() {
        let store = ViolationStore::new(100);

        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "test",
            "rm -rf /",
        ));
        store.record(Violation::warning(
            ViolationType::SensitiveFileAccess,
            "test",
            "cat ~/.ssh/id_rsa",
        ));

        assert_eq!(store.total_count(), 2);
        assert_eq!(store.blocked_count(), 1);
    }

    #[test]
    fn test_store_max_capacity() {
        let store = ViolationStore::new(3);

        for i in 0..5 {
            store.record(Violation::warning(
                ViolationType::PathAccessDenied,
                format!("test {}", i),
                format!("cmd {}", i),
            ));
        }

        assert_eq!(store.total_count(), 3);
        // Should have kept the most recent
        let violations = store.get_all();
        assert!(violations[0].message.contains("2"));
    }

    #[test]
    fn test_store_get_by_type() {
        let store = ViolationStore::new(100);

        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "test1",
            "rm /",
        ));
        store.record(Violation::blocked(
            ViolationType::HeredocInjection,
            "test2",
            "cat << $X",
        ));
        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "test3",
            "rm /usr",
        ));

        let removals = store.get_by_type(ViolationType::CriticalPathRemoval);
        assert_eq!(removals.len(), 2);
    }

    #[test]
    fn test_store_get_by_severity() {
        let store = ViolationStore::new(100);

        store.record(
            Violation::warning(ViolationType::DisallowedTempWrite, "low", "write /tmp/x")
                .with_severity(ViolationSeverity::Low),
        );
        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "critical",
            "rm /",
        ));

        let high_and_above = store.get_by_severity(ViolationSeverity::High);
        assert_eq!(high_and_above.len(), 1);
        assert!(high_and_above[0].message.contains("critical"));
    }

    #[test]
    fn test_store_has_critical() {
        let store = ViolationStore::new(100);

        store.record(Violation::warning(
            ViolationType::PathAccessDenied,
            "test",
            "cmd",
        ));
        assert!(!store.has_critical());

        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "test",
            "rm /",
        ));
        assert!(store.has_critical());
    }

    #[test]
    fn test_store_summary() {
        let store = ViolationStore::new(100);

        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "test",
            "rm /",
        ));
        store.record(Violation::warning(
            ViolationType::PathAccessDenied,
            "test",
            "read /etc",
        ));

        let summary = store.summary();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.blocked, 1);
        assert!(summary.has_critical);
    }

    #[test]
    fn test_store_clear() {
        let store = ViolationStore::new(100);

        store.record(Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "test",
            "rm /",
        ));
        assert_eq!(store.total_count(), 1);

        store.clear();
        assert_eq!(store.total_count(), 0);
    }

    #[test]
    fn test_shared_store() {
        let store = ViolationStore::shared(100);

        store.record(Violation::warning(
            ViolationType::PathAccessDenied,
            "test",
            "cmd",
        ));

        // Clone the Arc
        let store2 = Arc::clone(&store);
        assert_eq!(store2.total_count(), 1);
    }
}
