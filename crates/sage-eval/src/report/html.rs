//! HTML report generation

use anyhow::Result;

use crate::metrics::EvalMetrics;

/// HTML report generator
pub struct HtmlReporter;

impl HtmlReporter {
    /// Generate an HTML report
    pub fn generate(metrics: &EvalMetrics) -> Result<String> {
        let mut html = String::new();

        // HTML header
        html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Sage Evaluation Report</title>
    <style>
        :root {
            --bg-primary: #1a1a2e;
            --bg-secondary: #16213e;
            --bg-card: #0f3460;
            --text-primary: #eee;
            --text-secondary: #aaa;
            --accent: #e94560;
            --success: #4ade80;
            --warning: #fbbf24;
            --error: #f87171;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
            padding: 2rem;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { color: var(--accent); margin-bottom: 1rem; }
        h2 { color: var(--text-primary); margin: 2rem 0 1rem; border-bottom: 2px solid var(--accent); padding-bottom: 0.5rem; }
        .meta { background: var(--bg-secondary); padding: 1rem; border-radius: 8px; margin-bottom: 2rem; }
        .meta-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; }
        .meta-item { }
        .meta-label { color: var(--text-secondary); font-size: 0.875rem; }
        .meta-value { font-size: 1.125rem; font-weight: 600; }
        .summary-cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
        .card { background: var(--bg-card); padding: 1.5rem; border-radius: 8px; text-align: center; }
        .card-value { font-size: 2rem; font-weight: bold; color: var(--accent); }
        .card-label { color: var(--text-secondary); font-size: 0.875rem; }
        table { width: 100%; border-collapse: collapse; margin-bottom: 2rem; }
        th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid var(--bg-secondary); }
        th { background: var(--bg-secondary); color: var(--text-primary); font-weight: 600; }
        tr:hover { background: var(--bg-secondary); }
        .status-pass { color: var(--success); }
        .status-fail { color: var(--error); }
        .status-timeout { color: var(--warning); }
        .status-error { color: var(--error); }
        .progress-bar { background: var(--bg-secondary); border-radius: 4px; height: 8px; overflow: hidden; }
        .progress-fill { background: var(--success); height: 100%; transition: width 0.3s; }
        .category-card { background: var(--bg-card); padding: 1rem; border-radius: 8px; margin-bottom: 1rem; }
        .category-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
        .category-name { font-weight: 600; }
        .category-rate { color: var(--accent); }
    </style>
</head>
<body>
    <div class="container">
"#);

        // Title
        html.push_str("        <h1>Sage Evaluation Report</h1>\n");

        // Metadata
        html.push_str(r#"        <div class="meta">
            <div class="meta-grid">
"#);
        html.push_str(&format!(
            r#"                <div class="meta-item">
                    <div class="meta-label">Model</div>
                    <div class="meta-value">{}</div>
                </div>
"#,
            metrics.model
        ));
        html.push_str(&format!(
            r#"                <div class="meta-item">
                    <div class="meta-label">Provider</div>
                    <div class="meta-value">{}</div>
                </div>
"#,
            metrics.provider
        ));
        html.push_str(&format!(
            r#"                <div class="meta-item">
                    <div class="meta-label">Sage Version</div>
                    <div class="meta-value">{}</div>
                </div>
"#,
            metrics.sage_version
        ));
        html.push_str(&format!(
            r#"                <div class="meta-item">
                    <div class="meta-label">Timestamp</div>
                    <div class="meta-value">{}</div>
                </div>
"#,
            metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        html.push_str("            </div>\n        </div>\n");

        // Summary Cards
        html.push_str(r#"        <h2>Summary</h2>
        <div class="summary-cards">
"#);
        html.push_str(&format!(
            r#"            <div class="card">
                <div class="card-value">{:.1}%</div>
                <div class="card-label">Pass@1 Rate</div>
            </div>
"#,
            metrics.pass_at_1.rate * 100.0
        ));
        html.push_str(&format!(
            r#"            <div class="card">
                <div class="card-value">{}/{}</div>
                <div class="card-label">Tasks Passed</div>
            </div>
"#,
            metrics.pass_at_1.passed, metrics.pass_at_1.total
        ));
        html.push_str(&format!(
            r#"            <div class="card">
                <div class="card-value">{:.1}</div>
                <div class="card-label">Avg Turns</div>
            </div>
"#,
            metrics.turn_metrics.avg_turns
        ));
        html.push_str(&format!(
            r#"            <div class="card">
                <div class="card-value">{}</div>
                <div class="card-label">Total Tokens</div>
            </div>
"#,
            metrics.token_efficiency.total_tokens
        ));
        html.push_str(&format!(
            r#"            <div class="card">
                <div class="card-value">{:.1}s</div>
                <div class="card-label">Total Time</div>
            </div>
"#,
            metrics.total_execution_time_secs
        ));
        html.push_str("        </div>\n");

        // By Category
        html.push_str("        <h2>Results by Category</h2>\n");
        for (name, cat_metrics) in &metrics.by_category {
            let passed = (cat_metrics.pass_rate * cat_metrics.task_count as f64) as u32;
            html.push_str(&format!(
                r#"        <div class="category-card">
            <div class="category-header">
                <span class="category-name">{}</span>
                <span class="category-rate">{}/{} ({:.1}%)</span>
            </div>
            <div class="progress-bar">
                <div class="progress-fill" style="width: {:.1}%"></div>
            </div>
        </div>
"#,
                name,
                passed,
                cat_metrics.task_count,
                cat_metrics.pass_rate * 100.0,
                cat_metrics.pass_rate * 100.0
            ));
        }

        // Task Results Table
        html.push_str(r#"        <h2>Task Results</h2>
        <table>
            <thead>
                <tr>
                    <th>Task</th>
                    <th>Category</th>
                    <th>Difficulty</th>
                    <th>Status</th>
                    <th>Turns</th>
                    <th>Tokens</th>
                    <th>Time</th>
                </tr>
            </thead>
            <tbody>
"#);

        for result in &metrics.task_results {
            let (status_class, status_text) = match result.status {
                crate::metrics::TaskStatus::Passed => ("status-pass", "PASS"),
                crate::metrics::TaskStatus::Failed => ("status-fail", "FAIL"),
                crate::metrics::TaskStatus::Timeout => ("status-timeout", "TIMEOUT"),
                crate::metrics::TaskStatus::Error => ("status-error", "ERROR"),
                crate::metrics::TaskStatus::Skipped => ("status-timeout", "SKIP"),
            };

            html.push_str(&format!(
                r#"                <tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td class="{}">{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{:.1}s</td>
                </tr>
"#,
                result.task_name,
                result.category,
                result.difficulty,
                status_class,
                status_text,
                result.turns,
                result.total_tokens,
                result.execution_time_secs
            ));
        }

        html.push_str("            </tbody>\n        </table>\n");

        // Footer
        html.push_str(r#"    </div>
</body>
</html>
"#);

        Ok(html)
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
    fn test_html_generation() {
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

        let html = HtmlReporter::generate(&metrics).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Sage Evaluation Report"));
        assert!(html.contains("test-model"));
    }
}
