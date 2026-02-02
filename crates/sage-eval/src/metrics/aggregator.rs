//! Metrics aggregation for evaluation results
//!
//! Aggregates individual task results into summary metrics.

use std::collections::HashMap;

use chrono::Utc;

use super::types::{
    CategoryMetrics, CostEstimate, DifficultyMetrics, EvalMetrics, PassAtK, TaskResult,
    TokenEfficiency, TurnMetrics,
};
use crate::tasks::{Difficulty, TaskCategory};

/// Aggregator for computing summary metrics from task results
pub struct MetricsAggregator {
    /// Model name
    model: String,
    /// Provider name
    provider: String,
    /// Sage version
    sage_version: String,
}

impl MetricsAggregator {
    /// Create a new aggregator
    pub fn new(model: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            provider: provider.into(),
            sage_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Aggregate results into evaluation metrics
    pub fn aggregate(&self, results: Vec<TaskResult>, total_time_secs: f64) -> EvalMetrics {
        let pass_at_1 = self.compute_pass_at_k(&results, 1);
        let pass_at_3 = self.compute_pass_at_k_multi_attempt(&results, 3);

        let token_efficiency = TokenEfficiency::from_results(&results);
        let turn_metrics = TurnMetrics::from_results(&results);
        let by_category = self.compute_category_metrics(&results);

        // Compute cost estimate
        let cost_estimate = CostEstimate::from_provider(
            &self.provider,
            &self.model,
            token_efficiency.total_input_tokens,
            token_efficiency.total_output_tokens,
        );

        EvalMetrics {
            pass_at_1,
            pass_at_3,
            token_efficiency,
            turn_metrics,
            cost_estimate,
            by_category,
            task_results: results,
            total_execution_time_secs: total_time_secs,
            timestamp: Utc::now(),
            model: self.model.clone(),
            provider: self.provider.clone(),
            sage_version: self.sage_version.clone(),
        }
    }

    /// Compute Pass@K for single-attempt results
    fn compute_pass_at_k(&self, results: &[TaskResult], k: u32) -> PassAtK {
        // Group by task_id, take first k attempts
        let mut task_results: HashMap<String, Vec<&TaskResult>> = HashMap::new();
        for result in results {
            task_results
                .entry(result.task_id.clone())
                .or_default()
                .push(result);
        }

        let mut passed = 0;
        let total = task_results.len() as u32;

        for (_task_id, attempts) in task_results {
            // Sort by attempt number
            let mut attempts = attempts;
            attempts.sort_by_key(|r| r.attempt);

            // Check if any of the first k attempts passed
            let passed_in_k = attempts
                .iter()
                .take(k as usize)
                .any(|r| r.passed());

            if passed_in_k {
                passed += 1;
            }
        }

        PassAtK::new(k, passed, total)
    }

    /// Compute Pass@K when we have multiple attempts per task
    fn compute_pass_at_k_multi_attempt(&self, results: &[TaskResult], k: u32) -> Option<PassAtK> {
        // Check if we have multiple attempts
        let max_attempt = results.iter().map(|r| r.attempt).max().unwrap_or(1);
        if max_attempt < k {
            return None;
        }

        Some(self.compute_pass_at_k(results, k))
    }

    /// Compute metrics by category
    fn compute_category_metrics(
        &self,
        results: &[TaskResult],
    ) -> HashMap<String, CategoryMetrics> {
        let mut by_category: HashMap<TaskCategory, Vec<&TaskResult>> = HashMap::new();

        for result in results {
            by_category
                .entry(result.category)
                .or_default()
                .push(result);
        }

        let mut metrics = HashMap::new();

        for (category, cat_results) in by_category {
            let task_count = cat_results.len() as u32;
            let passed = cat_results.iter().filter(|r| r.passed()).count() as u32;
            let pass_rate = if task_count > 0 {
                passed as f64 / task_count as f64
            } else {
                0.0
            };

            let total_turns: u32 = cat_results.iter().map(|r| r.turns).sum();
            let avg_turns = if task_count > 0 {
                total_turns as f64 / task_count as f64
            } else {
                0.0
            };

            let total_tokens: u64 = cat_results.iter().map(|r| r.total_tokens).sum();
            let avg_tokens = if task_count > 0 {
                total_tokens as f64 / task_count as f64
            } else {
                0.0
            };

            let by_difficulty = self.compute_difficulty_metrics(&cat_results);

            metrics.insert(
                category.dir_name().to_string(),
                CategoryMetrics {
                    category,
                    task_count,
                    pass_rate,
                    avg_turns,
                    avg_tokens,
                    by_difficulty,
                },
            );
        }

        metrics
    }

    /// Compute metrics by difficulty within a category
    fn compute_difficulty_metrics(
        &self,
        results: &[&TaskResult],
    ) -> HashMap<String, DifficultyMetrics> {
        let mut by_difficulty: HashMap<Difficulty, Vec<&&TaskResult>> = HashMap::new();

        for result in results {
            by_difficulty
                .entry(result.difficulty)
                .or_default()
                .push(result);
        }

        let mut metrics = HashMap::new();

        for (difficulty, diff_results) in by_difficulty {
            let task_count = diff_results.len() as u32;
            let passed = diff_results.iter().filter(|r| r.passed()).count() as u32;
            let pass_rate = if task_count > 0 {
                passed as f64 / task_count as f64
            } else {
                0.0
            };

            metrics.insert(
                difficulty.display_name().to_lowercase(),
                DifficultyMetrics {
                    task_count,
                    passed,
                    pass_rate,
                },
            );
        }

        metrics
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new("unknown", "unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::TaskStatus;

    fn make_result(
        task_id: &str,
        category: TaskCategory,
        difficulty: Difficulty,
        passed: bool,
        attempt: u32,
    ) -> TaskResult {
        let mut result = TaskResult::new(
            task_id,
            format!("Task {}", task_id),
            category,
            difficulty,
            if passed {
                TaskStatus::Passed
            } else {
                TaskStatus::Failed
            },
        );
        result.attempt = attempt;
        result.turns = 5;
        result.total_tokens = 1000;
        result
    }

    #[test]
    fn test_pass_at_1() {
        let results = vec![
            make_result("t1", TaskCategory::CodeGeneration, Difficulty::Easy, true, 1),
            make_result("t2", TaskCategory::CodeGeneration, Difficulty::Easy, false, 1),
            make_result("t3", TaskCategory::CodeGeneration, Difficulty::Medium, true, 1),
        ];

        let aggregator = MetricsAggregator::new("test-model", "test-provider");
        let metrics = aggregator.aggregate(results, 10.0);

        assert_eq!(metrics.pass_at_1.passed, 2);
        assert_eq!(metrics.pass_at_1.total, 3);
        assert!((metrics.pass_at_1.rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_pass_at_3_with_retries() {
        let results = vec![
            // Task 1: fails first two, passes third
            make_result("t1", TaskCategory::CodeGeneration, Difficulty::Easy, false, 1),
            make_result("t1", TaskCategory::CodeGeneration, Difficulty::Easy, false, 2),
            make_result("t1", TaskCategory::CodeGeneration, Difficulty::Easy, true, 3),
            // Task 2: passes first try
            make_result("t2", TaskCategory::CodeGeneration, Difficulty::Easy, true, 1),
            make_result("t2", TaskCategory::CodeGeneration, Difficulty::Easy, true, 2),
            make_result("t2", TaskCategory::CodeGeneration, Difficulty::Easy, true, 3),
            // Task 3: never passes
            make_result("t3", TaskCategory::CodeGeneration, Difficulty::Easy, false, 1),
            make_result("t3", TaskCategory::CodeGeneration, Difficulty::Easy, false, 2),
            make_result("t3", TaskCategory::CodeGeneration, Difficulty::Easy, false, 3),
        ];

        let aggregator = MetricsAggregator::new("test-model", "test-provider");
        let metrics = aggregator.aggregate(results, 30.0);

        // Pass@1: only t2 passes on first try
        assert_eq!(metrics.pass_at_1.passed, 1);
        assert_eq!(metrics.pass_at_1.total, 3);

        // Pass@3: t1 and t2 pass within 3 attempts
        let pass_at_3 = metrics.pass_at_3.unwrap();
        assert_eq!(pass_at_3.passed, 2);
        assert_eq!(pass_at_3.total, 3);
    }

    #[test]
    fn test_category_metrics() {
        let results = vec![
            make_result("t1", TaskCategory::CodeGeneration, Difficulty::Easy, true, 1),
            make_result("t2", TaskCategory::CodeGeneration, Difficulty::Medium, false, 1),
            make_result("t3", TaskCategory::BugFixing, Difficulty::Easy, true, 1),
        ];

        let aggregator = MetricsAggregator::new("test-model", "test-provider");
        let metrics = aggregator.aggregate(results, 15.0);

        let code_gen = metrics.by_category.get("code_generation").unwrap();
        assert_eq!(code_gen.task_count, 2);
        assert_eq!(code_gen.pass_rate, 0.5);

        let bug_fix = metrics.by_category.get("bug_fixing").unwrap();
        assert_eq!(bug_fix.task_count, 1);
        assert_eq!(bug_fix.pass_rate, 1.0);
    }
}
