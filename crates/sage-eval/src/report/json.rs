//! JSON report generation

use anyhow::Result;

use crate::metrics::EvalMetrics;

/// JSON report generator
pub struct JsonReporter;

impl JsonReporter {
    /// Generate a JSON report
    pub fn generate(metrics: &EvalMetrics) -> Result<String> {
        let json = serde_json::to_string_pretty(metrics)?;
        Ok(json)
    }

    /// Generate a compact JSON report (no pretty printing)
    pub fn generate_compact(metrics: &EvalMetrics) -> Result<String> {
        let json = serde_json::to_string(metrics)?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{PassAtK, TaskResult, TaskStatus, TokenEfficiency, TurnMetrics};
    use crate::tasks::{Difficulty, TaskCategory};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_metrics() -> EvalMetrics {
        let results = vec![TaskResult::new(
            "test-001",
            "Test Task",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            TaskStatus::Passed,
        )];

        EvalMetrics {
            pass_at_1: PassAtK::new(1, 1, 1),
            pass_at_3: None,
            token_efficiency: TokenEfficiency::from_results(&results),
            turn_metrics: TurnMetrics::from_results(&results),
            by_category: HashMap::new(),
            task_results: results,
            total_execution_time_secs: 10.0,
            timestamp: Utc::now(),
            model: "test-model".to_string(),
            provider: "test-provider".to_string(),
            sage_version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn test_json_generation() {
        let metrics = create_test_metrics();
        let json = JsonReporter::generate(&metrics).unwrap();

        assert!(json.contains("test-model"));
        assert!(json.contains("pass_at_1"));
    }
}
