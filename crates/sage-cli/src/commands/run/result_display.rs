//! Result display logic for run command

use crate::console::CliConsole;
use sage_sdk::{ExecutionErrorKind, ExecutionOutcome, ExecutionResult};

/// Display execution result with detailed messages
pub fn display_result(result: &ExecutionResult, console: &CliConsole, verbose: bool) {
    match &result.outcome {
        ExecutionOutcome::Success(_) => {
            console.success("Task completed successfully!");
        }
        ExecutionOutcome::Failed { error, .. } => {
            display_error(error, console);
        }
        ExecutionOutcome::Interrupted { .. } => {
            console.warn("🛑 Task interrupted by user (Ctrl+C)");
            console.info("Task was stopped gracefully.");
        }
        ExecutionOutcome::MaxStepsReached { .. } => {
            console.warn("⚠ Task reached maximum steps without completion");
            console.info("Consider breaking down the task or increasing max_steps");
        }
        ExecutionOutcome::UserCancelled {
            pending_question, ..
        } => {
            console.warn("⊘ Task cancelled by user");
            if let Some(question) = pending_question {
                console.info(&format!("Pending question: {}", question));
            }
        }
        ExecutionOutcome::NeedsUserInput { last_response, .. } => {
            console.info("💬 AI is waiting for user input");
            if !last_response.is_empty() {
                console.info(&format!("Last response: {}", last_response));
            }
            console.info(
                "ℹ Use interactive mode (sage interactive) for multi-turn conversations",
            );
        }
    }

    // Display final result if available
    if let Some(final_result) = result.final_result() {
        console.print_header("Final Result");
        println!("{}", final_result);
    }

    // Display trajectory path if saved
    if let Some(trajectory_path) = result.trajectory_path() {
        console.info(&format!(
            "Trajectory saved to: {}",
            trajectory_path.display()
        ));
    }

    // Print statistics if verbose
    if verbose {
        display_statistics(result, console);
    }
}

/// Display error details
fn display_error(error: &sage_sdk::ExecutionError, console: &CliConsole) {
    console.error("Task execution failed!");
    console.error(&format!("  Error: {}", error.message));

    // Show error type
    match &error.kind {
        ExecutionErrorKind::Authentication => {
            console.error("  Type: Authentication Error");
        }
        ExecutionErrorKind::RateLimit => {
            console.warn("  Type: Rate Limit");
        }
        ExecutionErrorKind::ServiceUnavailable => {
            console.warn("  Type: Service Unavailable");
        }
        ExecutionErrorKind::ToolExecution { tool_name } => {
            console.error(&format!("  Type: Tool Error ({})", tool_name));
        }
        ExecutionErrorKind::Configuration => {
            console.error("  Type: Configuration Error");
        }
        ExecutionErrorKind::Network => {
            console.error("  Type: Network Error");
        }
        ExecutionErrorKind::Timeout => {
            console.warn("  Type: Timeout");
        }
        ExecutionErrorKind::InvalidRequest => {
            console.error("  Type: Invalid Request");
        }
        ExecutionErrorKind::Other => {}
    }

    // Show provider if available
    if let Some(provider) = &error.provider {
        console.info(&format!("  Provider: {}", provider));
    }

    // Show suggestion if available
    if let Some(suggestion) = &error.suggestion {
        console.info(&format!("  💡 {}", suggestion));
    }
}

/// Display execution statistics
fn display_statistics(result: &ExecutionResult, console: &CliConsole) {
    console.print_header("Execution Statistics");
    let stats = result.statistics();
    console.info(&format!("Successful steps: {}", stats.successful_steps));
    console.info(&format!("Failed steps: {}", stats.failed_steps));
    console.info(&format!("Tool calls: {}", stats.tool_calls));
    console.info(&format!("Total tokens: {}", stats.total_tokens));

    // Show cache statistics
    if stats.cache_creation_tokens.is_some() || stats.cache_read_tokens.is_some() {
        let mut cache_parts = Vec::new();
        if let Some(created) = stats.cache_creation_tokens {
            cache_parts.push(format!("{} created", created));
        }
        if let Some(read) = stats.cache_read_tokens {
            cache_parts.push(format!("{} read", read));
        }
        console.info(&format!("Cache tokens: {}", cache_parts.join(", ")));
    }

    if !stats.tool_usage.is_empty() {
        console.info("Tool usage:");
        for (tool, count) in &stats.tool_usage {
            console.info(&format!("  {}: {} times", tool, count));
        }
    }
}

/// Display token usage information
pub fn display_token_usage(result: &ExecutionResult, console: &CliConsole) {
    let usage = &result.execution().total_usage;
    let mut token_info = format!("Total tokens: {}", usage.total_tokens);

    // Add cache metrics if available
    if usage.has_cache_metrics() {
        let mut cache_parts = Vec::new();
        if let Some(created) = usage.cache_creation_input_tokens {
            if created > 0 {
                cache_parts.push(format!("{} created", created));
            }
        }
        if let Some(read) = usage.cache_read_input_tokens {
            if read > 0 {
                cache_parts.push(format!("{} read", read));
            }
        }
        if !cache_parts.is_empty() {
            token_info.push_str(&format!(" (cache: {})", cache_parts.join(", ")));
        }
    }
    console.info(&token_info);
}
