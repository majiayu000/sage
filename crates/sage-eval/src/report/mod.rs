//! Report generation for evaluation results
//!
//! Generates reports in various formats (JSON, Markdown, HTML).

mod html;
mod json;
mod markdown;

pub use html::HtmlReporter;
pub use json::JsonReporter;
pub use markdown::MarkdownReporter;

use crate::metrics::EvalMetrics;
use anyhow::Result;

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Markdown,
    Html,
    Table,
}

impl ReportFormat {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(ReportFormat::Json),
            "markdown" | "md" => Some(ReportFormat::Markdown),
            "html" => Some(ReportFormat::Html),
            "table" => Some(ReportFormat::Table),
            _ => None,
        }
    }
}

/// Generate a report in the specified format
pub fn generate_report(metrics: &EvalMetrics, format: ReportFormat) -> Result<String> {
    match format {
        ReportFormat::Json => JsonReporter::generate(metrics),
        ReportFormat::Markdown => MarkdownReporter::generate(metrics),
        ReportFormat::Html => HtmlReporter::generate(metrics),
        ReportFormat::Table => generate_table(metrics),
    }
}

/// Generate a simple table report for terminal output
fn generate_table(metrics: &EvalMetrics) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "\n{:=<70}\n",
        "= Sage Evaluation Results "
    ));
    output.push_str(&format!(
        "Model: {} | Provider: {} | Sage: {}\n",
        metrics.model, metrics.provider, metrics.sage_version
    ));
    output.push_str(&format!(
        "Timestamp: {}\n",
        metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    output.push_str(&format!("{:=<70}\n\n", ""));

    // Summary
    output.push_str("SUMMARY\n");
    output.push_str(&format!("{:-<70}\n", ""));
    output.push_str(&format!(
        "Pass@1: {}/{} ({:.1}%)\n",
        metrics.pass_at_1.passed,
        metrics.pass_at_1.total,
        metrics.pass_at_1.rate * 100.0
    ));

    if let Some(ref pass_at_3) = metrics.pass_at_3 {
        output.push_str(&format!(
            "Pass@3: {}/{} ({:.1}%)\n",
            pass_at_3.passed, pass_at_3.total, pass_at_3.rate * 100.0
        ));
    }

    output.push_str(&format!(
        "Total Time: {:.1}s\n",
        metrics.total_execution_time_secs
    ));
    output.push_str(&format!(
        "Avg Turns: {:.1}\n",
        metrics.turn_metrics.avg_turns
    ));
    output.push_str(&format!(
        "Total Tokens: {}\n",
        metrics.token_efficiency.total_tokens
    ));

    // Cost estimate
    output.push_str(&format!(
        "Estimated Cost: {} (input: ${:.4}, output: ${:.4})\n\n",
        metrics.cost_estimate.format_cost(),
        metrics.cost_estimate.input_cost_usd,
        metrics.cost_estimate.output_cost_usd
    ));

    // By Category
    output.push_str("BY CATEGORY\n");
    output.push_str(&format!("{:-<70}\n", ""));
    output.push_str(&format!(
        "{:<20} {:>8} {:>10} {:>10} {:>12}\n",
        "Category", "Tasks", "Passed", "Rate", "Avg Turns"
    ));
    output.push_str(&format!("{:-<70}\n", ""));

    for (name, cat_metrics) in &metrics.by_category {
        let passed = (cat_metrics.pass_rate * cat_metrics.task_count as f64) as u32;
        output.push_str(&format!(
            "{:<20} {:>8} {:>10} {:>9.1}% {:>12.1}\n",
            name,
            cat_metrics.task_count,
            passed,
            cat_metrics.pass_rate * 100.0,
            cat_metrics.avg_turns
        ));
    }

    output.push_str(&format!("{:-<70}\n\n", ""));

    // Token breakdown
    output.push_str("TOKEN USAGE & COST\n");
    output.push_str(&format!("{:-<70}\n", ""));
    output.push_str(&format!(
        "Input Tokens:  {:>12} × ${:.2}/1M = ${:.4}\n",
        metrics.token_efficiency.total_input_tokens,
        metrics.cost_estimate.input_cost_per_million,
        metrics.cost_estimate.input_cost_usd
    ));
    output.push_str(&format!(
        "Output Tokens: {:>12} × ${:.2}/1M = ${:.4}\n",
        metrics.token_efficiency.total_output_tokens,
        metrics.cost_estimate.output_cost_per_million,
        metrics.cost_estimate.output_cost_usd
    ));
    output.push_str(&format!(
        "Total:         {:>12}              {}\n",
        metrics.token_efficiency.total_tokens,
        metrics.cost_estimate.format_cost()
    ));
    output.push_str(&format!("{:-<70}\n\n", ""));

    // Task Results
    output.push_str("TASK RESULTS\n");
    output.push_str(&format!("{:-<70}\n", ""));
    output.push_str(&format!(
        "{:<30} {:>10} {:>8} {:>10} {:>8}\n",
        "Task", "Status", "Turns", "Tokens", "Time"
    ));
    output.push_str(&format!("{:-<70}\n", ""));

    for result in &metrics.task_results {
        let status = match result.status {
            crate::metrics::TaskStatus::Passed => "PASS",
            crate::metrics::TaskStatus::Failed => "FAIL",
            crate::metrics::TaskStatus::Timeout => "TIMEOUT",
            crate::metrics::TaskStatus::Error => "ERROR",
            crate::metrics::TaskStatus::Skipped => "SKIP",
        };

        let task_name = if result.task_name.len() > 28 {
            format!("{}...", &result.task_name[..25])
        } else {
            result.task_name.clone()
        };

        output.push_str(&format!(
            "{:<30} {:>10} {:>8} {:>10} {:>7.1}s\n",
            task_name,
            status,
            result.turns,
            result.total_tokens,
            result.execution_time_secs
        ));
    }

    output.push_str(&format!("{:=<70}\n", ""));

    Ok(output)
}
