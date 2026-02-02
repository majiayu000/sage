//! Markdown report generation

use anyhow::Result;

use crate::metrics::EvalMetrics;

/// Markdown report generator
pub struct MarkdownReporter;

impl MarkdownReporter {
    /// Generate a Markdown report
    pub fn generate(metrics: &EvalMetrics) -> Result<String> {
        let mut md = String::new();

        // Title
        md.push_str("# Sage Evaluation Report\n\n");

        // Metadata
        md.push_str("## Overview\n\n");
        md.push_str(&format!("- **Model**: {}\n", metrics.model));
        md.push_str(&format!("- **Provider**: {}\n", metrics.provider));
        md.push_str(&format!("- **Sage Version**: {}\n", metrics.sage_version));
        md.push_str(&format!(
            "- **Timestamp**: {}\n",
            metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!(
            "- **Total Execution Time**: {:.1}s\n\n",
            metrics.total_execution_time_secs
        ));

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "| Metric | Value |\n|--------|-------|\n"
        ));
        md.push_str(&format!(
            "| Pass@1 | {}/{} ({:.1}%) |\n",
            metrics.pass_at_1.passed,
            metrics.pass_at_1.total,
            metrics.pass_at_1.rate * 100.0
        ));

        if let Some(ref pass_at_3) = metrics.pass_at_3 {
            md.push_str(&format!(
                "| Pass@3 | {}/{} ({:.1}%) |\n",
                pass_at_3.passed, pass_at_3.total, pass_at_3.rate * 100.0
            ));
        }

        md.push_str(&format!(
            "| Avg Turns | {:.1} |\n",
            metrics.turn_metrics.avg_turns
        ));
        md.push_str(&format!(
            "| Avg Turns (Success) | {:.1} |\n",
            metrics.turn_metrics.avg_turns_success
        ));
        md.push_str(&format!(
            "| Total Tokens | {} |\n",
            metrics.token_efficiency.total_tokens
        ));
        md.push_str(&format!(
            "| Avg Tokens/Success | {:.0} |\n\n",
            metrics.token_efficiency.avg_tokens_per_success
        ));

        // By Category
        md.push_str("## Results by Category\n\n");
        md.push_str("| Category | Tasks | Passed | Rate | Avg Turns | Avg Tokens |\n");
        md.push_str("|----------|-------|--------|------|-----------|------------|\n");

        for (name, cat_metrics) in &metrics.by_category {
            let passed = (cat_metrics.pass_rate * cat_metrics.task_count as f64) as u32;
            md.push_str(&format!(
                "| {} | {} | {} | {:.1}% | {:.1} | {:.0} |\n",
                name,
                cat_metrics.task_count,
                passed,
                cat_metrics.pass_rate * 100.0,
                cat_metrics.avg_turns,
                cat_metrics.avg_tokens
            ));
        }
        md.push('\n');

        // Task Results
        md.push_str("## Task Results\n\n");
        md.push_str("| Task | Category | Difficulty | Status | Turns | Tokens | Time |\n");
        md.push_str("|------|----------|------------|--------|-------|--------|------|\n");

        for result in &metrics.task_results {
            let status_emoji = match result.status {
                crate::metrics::TaskStatus::Passed => "✅",
                crate::metrics::TaskStatus::Failed => "❌",
                crate::metrics::TaskStatus::Timeout => "⏱️",
                crate::metrics::TaskStatus::Error => "💥",
                crate::metrics::TaskStatus::Skipped => "⏭️",
            };

            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {:.1}s |\n",
                result.task_name,
                result.category,
                result.difficulty,
                status_emoji,
                result.turns,
                result.total_tokens,
                result.execution_time_secs
            ));
        }
        md.push('\n');

        // Failed Tasks Details
        let failed: Vec<_> = metrics
            .task_results
            .iter()
            .filter(|r| !r.passed())
            .collect();

        if !failed.is_empty() {
            md.push_str("## Failed Tasks\n\n");

            for result in failed {
                md.push_str(&format!("### {}\n\n", result.task_name));
                md.push_str(&format!("- **ID**: {}\n", result.task_id));
                md.push_str(&format!("- **Status**: {:?}\n", result.status));

                if let Some(ref error) = result.error_message {
                    md.push_str(&format!("- **Error**: {}\n", error));
                }

                if let Some(ref output) = result.verifier_output {
                    md.push_str(&format!("\n```\n{}\n```\n", output));
                }
                md.push('\n');
            }
        }

        Ok(md)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{PassAtK, TaskResult, TaskStatus, TokenEfficiency, TurnMetrics};
    use crate::tasks::{Difficulty, TaskCategory};
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_markdown_generation() {
        let results = vec![TaskResult::new(
            "test-001",
            "Test Task",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            TaskStatus::Passed,
        )];

        let metrics = EvalMetrics {
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
        };

        let md = MarkdownReporter::generate(&metrics).unwrap();

        assert!(md.contains("# Sage Evaluation Report"));
        assert!(md.contains("test-model"));
        assert!(md.contains("Pass@1"));
    }
}
