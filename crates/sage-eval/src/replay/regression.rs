//! Regression detection for evaluation results
//!
//! Compares current results against baseline to detect regressions.

use serde::{Deserialize, Serialize};

use crate::metrics::EvalMetrics;

/// A detected regression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    /// Type of regression
    pub regression_type: RegressionType,

    /// Description of the regression
    pub description: String,

    /// Baseline value
    pub baseline_value: String,

    /// Current value
    pub current_value: String,

    /// Severity (0.0 - 1.0)
    pub severity: f64,
}

/// Type of regression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegressionType {
    /// Pass rate decreased
    PassRateDecrease,
    /// Token usage increased significantly
    TokenIncrease,
    /// Turn count increased significantly
    TurnIncrease,
    /// Execution time increased significantly
    TimeIncrease,
    /// Previously passing task now fails
    TaskRegression,
}

/// Regression detector configuration
#[derive(Debug, Clone)]
pub struct RegressionConfig {
    /// Threshold for pass rate decrease (e.g., 0.05 = 5%)
    pub pass_rate_threshold: f64,

    /// Threshold for token increase (e.g., 0.20 = 20%)
    pub token_threshold: f64,

    /// Threshold for turn increase (e.g., 0.20 = 20%)
    pub turn_threshold: f64,

    /// Threshold for time increase (e.g., 0.50 = 50%)
    pub time_threshold: f64,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            pass_rate_threshold: 0.05,
            token_threshold: 0.20,
            turn_threshold: 0.20,
            time_threshold: 0.50,
        }
    }
}

/// Detector for finding regressions between evaluation runs
pub struct RegressionDetector {
    config: RegressionConfig,
}

impl RegressionDetector {
    /// Create a new regression detector
    pub fn new(config: RegressionConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RegressionConfig::default())
    }

    /// Compare current metrics against baseline
    pub fn detect(&self, baseline: &EvalMetrics, current: &EvalMetrics) -> Vec<Regression> {
        let mut regressions = Vec::new();

        // Check pass rate
        if let Some(reg) = self.check_pass_rate(baseline, current) {
            regressions.push(reg);
        }

        // Check token usage
        if let Some(reg) = self.check_tokens(baseline, current) {
            regressions.push(reg);
        }

        // Check turn count
        if let Some(reg) = self.check_turns(baseline, current) {
            regressions.push(reg);
        }

        // Check execution time
        if let Some(reg) = self.check_time(baseline, current) {
            regressions.push(reg);
        }

        // Check individual task regressions
        regressions.extend(self.check_task_regressions(baseline, current));

        regressions
    }

    fn check_pass_rate(&self, baseline: &EvalMetrics, current: &EvalMetrics) -> Option<Regression> {
        let baseline_rate = baseline.pass_at_1.rate;
        let current_rate = current.pass_at_1.rate;

        if baseline_rate - current_rate > self.config.pass_rate_threshold {
            let severity = (baseline_rate - current_rate) / baseline_rate.max(0.01);
            Some(Regression {
                regression_type: RegressionType::PassRateDecrease,
                description: format!(
                    "Pass rate decreased from {:.1}% to {:.1}%",
                    baseline_rate * 100.0,
                    current_rate * 100.0
                ),
                baseline_value: format!("{:.1}%", baseline_rate * 100.0),
                current_value: format!("{:.1}%", current_rate * 100.0),
                severity: severity.min(1.0),
            })
        } else {
            None
        }
    }

    fn check_tokens(&self, baseline: &EvalMetrics, current: &EvalMetrics) -> Option<Regression> {
        let baseline_tokens = baseline.token_efficiency.avg_tokens_per_success;
        let current_tokens = current.token_efficiency.avg_tokens_per_success;

        if baseline_tokens > 0.0 {
            let increase = (current_tokens - baseline_tokens) / baseline_tokens;
            if increase > self.config.token_threshold {
                Some(Regression {
                    regression_type: RegressionType::TokenIncrease,
                    description: format!(
                        "Average tokens per success increased by {:.1}%",
                        increase * 100.0
                    ),
                    baseline_value: format!("{:.0}", baseline_tokens),
                    current_value: format!("{:.0}", current_tokens),
                    severity: (increase / 2.0).min(1.0),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    fn check_turns(&self, baseline: &EvalMetrics, current: &EvalMetrics) -> Option<Regression> {
        let baseline_turns = baseline.turn_metrics.avg_turns_success;
        let current_turns = current.turn_metrics.avg_turns_success;

        if baseline_turns > 0.0 {
            let increase = (current_turns - baseline_turns) / baseline_turns;
            if increase > self.config.turn_threshold {
                Some(Regression {
                    regression_type: RegressionType::TurnIncrease,
                    description: format!(
                        "Average turns per success increased by {:.1}%",
                        increase * 100.0
                    ),
                    baseline_value: format!("{:.1}", baseline_turns),
                    current_value: format!("{:.1}", current_turns),
                    severity: (increase / 2.0).min(1.0),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    fn check_time(&self, baseline: &EvalMetrics, current: &EvalMetrics) -> Option<Regression> {
        let baseline_time = baseline.total_execution_time_secs;
        let current_time = current.total_execution_time_secs;

        if baseline_time > 0.0 {
            let increase = (current_time - baseline_time) / baseline_time;
            if increase > self.config.time_threshold {
                Some(Regression {
                    regression_type: RegressionType::TimeIncrease,
                    description: format!(
                        "Total execution time increased by {:.1}%",
                        increase * 100.0
                    ),
                    baseline_value: format!("{:.1}s", baseline_time),
                    current_value: format!("{:.1}s", current_time),
                    severity: (increase / 3.0).min(1.0),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    fn check_task_regressions(
        &self,
        baseline: &EvalMetrics,
        current: &EvalMetrics,
    ) -> Vec<Regression> {
        let mut regressions = Vec::new();

        // Build map of baseline results
        let baseline_map: std::collections::HashMap<_, _> = baseline
            .task_results
            .iter()
            .map(|r| (&r.task_id, r.passed()))
            .collect();

        // Check each current result
        for result in &current.task_results {
            if let Some(&baseline_passed) = baseline_map.get(&result.task_id) {
                if baseline_passed && !result.passed() {
                    regressions.push(Regression {
                        regression_type: RegressionType::TaskRegression,
                        description: format!(
                            "Task '{}' was passing but now fails",
                            result.task_name
                        ),
                        baseline_value: "PASS".to_string(),
                        current_value: format!("{:?}", result.status),
                        severity: 0.8,
                    });
                }
            }
        }

        regressions
    }

    /// Generate a summary of regressions
    pub fn summarize(regressions: &[Regression]) -> String {
        if regressions.is_empty() {
            return "No regressions detected.".to_string();
        }

        let mut summary = format!("Found {} regression(s):\n", regressions.len());

        for (i, reg) in regressions.iter().enumerate() {
            summary.push_str(&format!(
                "  {}. [{:?}] {} (severity: {:.0}%)\n",
                i + 1,
                reg.regression_type,
                reg.description,
                reg.severity * 100.0
            ));
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{PassAtK, TaskResult, TaskStatus, TokenEfficiency, TurnMetrics};
    use crate::tasks::{Difficulty, TaskCategory};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_metrics(pass_rate: f64, avg_tokens: f64, avg_turns: f64) -> EvalMetrics {
        let total = 10;
        let passed = (pass_rate * total as f64) as u32;

        EvalMetrics {
            pass_at_1: PassAtK::new(1, passed, total),
            pass_at_3: None,
            token_efficiency: TokenEfficiency {
                avg_tokens_per_success: avg_tokens,
                avg_tokens_per_turn: avg_tokens / avg_turns.max(1.0),
                total_tokens: (avg_tokens * total as f64) as u64,
                total_input_tokens: 0,
                total_output_tokens: 0,
                input_output_ratio: 0.0,
            },
            turn_metrics: TurnMetrics {
                avg_turns,
                avg_turns_success: avg_turns,
                min_turns: 1,
                max_turns: 10,
                total_turns: (avg_turns * total as f64) as u32,
            },
            by_category: HashMap::new(),
            task_results: Vec::new(),
            total_execution_time_secs: 100.0,
            timestamp: Utc::now(),
            model: "test".to_string(),
            provider: "test".to_string(),
            sage_version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn test_no_regression() {
        let detector = RegressionDetector::with_defaults();
        let baseline = make_metrics(0.8, 1000.0, 5.0);
        let current = make_metrics(0.8, 1000.0, 5.0);

        let regressions = detector.detect(&baseline, &current);
        assert!(regressions.is_empty());
    }

    #[test]
    fn test_pass_rate_regression() {
        let detector = RegressionDetector::with_defaults();
        let baseline = make_metrics(0.8, 1000.0, 5.0);
        let current = make_metrics(0.6, 1000.0, 5.0);

        let regressions = detector.detect(&baseline, &current);
        assert!(!regressions.is_empty());
        assert!(regressions
            .iter()
            .any(|r| r.regression_type == RegressionType::PassRateDecrease));
    }

    #[test]
    fn test_token_regression() {
        let detector = RegressionDetector::with_defaults();
        let baseline = make_metrics(0.8, 1000.0, 5.0);
        let current = make_metrics(0.8, 1500.0, 5.0); // 50% increase

        let regressions = detector.detect(&baseline, &current);
        assert!(regressions
            .iter()
            .any(|r| r.regression_type == RegressionType::TokenIncrease));
    }
}
