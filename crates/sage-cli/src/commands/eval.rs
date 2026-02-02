//! Evaluation commands for running self-evaluation tests

use std::path::PathBuf;

use anyhow::Result;
use sage_eval::report::{generate_report, ReportFormat};
use sage_eval::runner::{EvalConfig, EvalExecutor, EvalProgress};
use sage_eval::tasks::{TaskCategory, TaskLoader};

/// Run evaluation tasks
pub async fn run(
    categories: Option<Vec<String>>,
    task_ids: Option<Vec<String>>,
    attempts: u32,
    output: Option<PathBuf>,
    format: String,
    config_file: String,
    verbose: bool,
) -> Result<()> {
    let mut eval_config = EvalConfig::new(&config_file)
        .with_attempts(attempts)
        .with_output_dir(output.unwrap_or_else(|| PathBuf::from(".")));

    if let Some(cats) = categories {
        eval_config = eval_config.with_categories(cats);
    }

    if let Some(ids) = task_ids {
        eval_config = eval_config.with_task_ids(ids);
    }

    if verbose {
        eval_config = eval_config.verbose();
    }

    let mut executor = EvalExecutor::new(eval_config)?;

    // Set up progress callback
    executor.set_progress_callback(Box::new(|progress: EvalProgress| {
        println!(
            "[{}/{}] {} - {} (attempt {})",
            progress.current + 1,
            progress.total,
            progress.task_id,
            progress.message,
            progress.attempt
        );
    }));

    println!("Starting evaluation...\n");

    let metrics = executor.run_all().await?;

    // Generate report
    let report_format = ReportFormat::from_str(&format).unwrap_or(ReportFormat::Table);
    let report = generate_report(&metrics, report_format)?;
    println!("{}", report);

    // Summary
    println!(
        "\nEvaluation complete: {}/{} tasks passed ({:.1}%)",
        metrics.passed_count(),
        metrics.total_count(),
        metrics.overall_pass_rate() * 100.0
    );

    Ok(())
}

/// List available evaluation tasks
pub async fn list(categories: Option<Vec<String>>) -> Result<()> {
    let loader = TaskLoader::builtin();

    let tasks = if let Some(cats) = categories {
        let cat_enums: Vec<TaskCategory> = cats
            .iter()
            .filter_map(|c| match c.as_str() {
                "code_generation" => Some(TaskCategory::CodeGeneration),
                "code_editing" => Some(TaskCategory::CodeEditing),
                "bug_fixing" => Some(TaskCategory::BugFixing),
                "refactoring" => Some(TaskCategory::Refactoring),
                "multi_file" => Some(TaskCategory::MultiFile),
                _ => None,
            })
            .collect();
        loader.load_categories(&cat_enums)?
    } else {
        loader.load_all()?
    };

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    println!("Available evaluation tasks:\n");
    println!(
        "{:<25} {:<30} {:<15} {:<10}",
        "ID", "Name", "Category", "Difficulty"
    );
    println!("{:-<80}", "");

    for task in &tasks {
        println!(
            "{:<25} {:<30} {:<15} {:<10}",
            task.id,
            if task.name.len() > 28 {
                format!("{}...", &task.name[..25])
            } else {
                task.name.clone()
            },
            task.category.dir_name(),
            task.difficulty.display_name()
        );
    }

    println!("\nTotal: {} tasks", tasks.len());

    // Show counts by category
    let counts = loader.count_by_category()?;
    println!("\nBy category:");
    for (cat, count) in counts {
        println!("  {}: {}", cat.display_name(), count);
    }

    Ok(())
}

/// Show evaluation report from a previous run
pub async fn report(input: PathBuf, format: String) -> Result<()> {
    let content = tokio::fs::read_to_string(&input).await?;
    let metrics: sage_eval::metrics::EvalMetrics = serde_json::from_str(&content)?;

    let report_format = ReportFormat::from_str(&format).unwrap_or(ReportFormat::Table);
    let report = generate_report(&metrics, report_format)?;
    println!("{}", report);

    Ok(())
}

/// Compare two evaluation results
pub async fn compare(baseline: PathBuf, current: PathBuf) -> Result<()> {
    let baseline_content = tokio::fs::read_to_string(&baseline).await?;
    let current_content = tokio::fs::read_to_string(&current).await?;

    let baseline_metrics: sage_eval::metrics::EvalMetrics =
        serde_json::from_str(&baseline_content)?;
    let current_metrics: sage_eval::metrics::EvalMetrics = serde_json::from_str(&current_content)?;

    // Use regression detector
    let detector = sage_eval::replay::RegressionDetector::with_defaults();
    let regressions = detector.detect(&baseline_metrics, &current_metrics);

    println!("Comparison: {} vs {}\n", baseline.display(), current.display());
    println!(
        "{:<20} {:>15} {:>15} {:>15}",
        "Metric", "Baseline", "Current", "Change"
    );
    println!("{:-<65}", "");

    // Pass rate
    let baseline_rate = baseline_metrics.pass_at_1.rate * 100.0;
    let current_rate = current_metrics.pass_at_1.rate * 100.0;
    let rate_change = current_rate - baseline_rate;
    println!(
        "{:<20} {:>14.1}% {:>14.1}% {:>+14.1}%",
        "Pass Rate", baseline_rate, current_rate, rate_change
    );

    // Avg turns
    let baseline_turns = baseline_metrics.turn_metrics.avg_turns;
    let current_turns = current_metrics.turn_metrics.avg_turns;
    let turns_change = current_turns - baseline_turns;
    println!(
        "{:<20} {:>15.1} {:>15.1} {:>+15.1}",
        "Avg Turns", baseline_turns, current_turns, turns_change
    );

    // Total tokens
    let baseline_tokens = baseline_metrics.token_efficiency.total_tokens;
    let current_tokens = current_metrics.token_efficiency.total_tokens;
    let tokens_change = current_tokens as i64 - baseline_tokens as i64;
    println!(
        "{:<20} {:>15} {:>15} {:>+15}",
        "Total Tokens", baseline_tokens, current_tokens, tokens_change
    );

    // Execution time
    let baseline_time = baseline_metrics.total_execution_time_secs;
    let current_time = current_metrics.total_execution_time_secs;
    let time_change = current_time - baseline_time;
    println!(
        "{:<20} {:>14.1}s {:>14.1}s {:>+14.1}s",
        "Execution Time", baseline_time, current_time, time_change
    );

    println!();

    // Show regressions
    if regressions.is_empty() {
        println!("No regressions detected.");
    } else {
        println!(
            "{}",
            sage_eval::replay::RegressionDetector::summarize(&regressions)
        );
    }

    Ok(())
}
