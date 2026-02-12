//! Execution outcome display for the unified command

use crate::console::CliConsole;
use sage_core::agent::ExecutionOutcome;
use std::path::PathBuf;

/// Display execution outcome
pub fn display_outcome(
    console: &CliConsole,
    outcome: &ExecutionOutcome,
    duration: std::time::Duration,
    session_path: Option<&PathBuf>,
) {
    match outcome {
        ExecutionOutcome::Success(_) => {
            console.success("Task completed successfully!");
        }
        ExecutionOutcome::Failed { error, .. } => {
            console.error("Task execution failed!");

            // Show error type for better debugging
            let error_type = match &error.kind {
                sage_core::agent::ExecutionErrorKind::Authentication => "Authentication Error",
                sage_core::agent::ExecutionErrorKind::RateLimit => "Rate Limit Error",
                sage_core::agent::ExecutionErrorKind::InvalidRequest => "Invalid Request",
                sage_core::agent::ExecutionErrorKind::ServiceUnavailable => "Service Unavailable",
                sage_core::agent::ExecutionErrorKind::ToolExecution { tool_name } => {
                    &format!("Tool Execution Error ({})", tool_name)
                }
                sage_core::agent::ExecutionErrorKind::Configuration => "Configuration Error",
                sage_core::agent::ExecutionErrorKind::Network => "Network Error",
                sage_core::agent::ExecutionErrorKind::Timeout => "Timeout Error",
                sage_core::agent::ExecutionErrorKind::Other => "Error",
            };
            console.error(&format!("[{}] {}", error_type, error.message));

            if let Some(provider) = &error.provider {
                console.info(&format!("Provider: {}", provider));
            }
            if let Some(suggestion) = &error.suggestion {
                console.info(&format!("Suggestion: {}", suggestion));
            }

            // Show session ID for debugging if available
            if let Some(path) = session_path {
                console.info(&format!("Session logs: {}", path.display()));
            }
        }
        ExecutionOutcome::Interrupted { .. } => {
            console.warn("Task interrupted by user (Ctrl+C)");
        }
        ExecutionOutcome::MaxStepsReached { .. } => {
            console.warn("Task reached maximum steps without completion");
            console.info("Consider breaking down the task or increasing max_steps");
        }
        ExecutionOutcome::UserCancelled {
            pending_question, ..
        } => {
            console.warn("Task cancelled by user");
            if let Some(question) = pending_question {
                console.info(&format!("Pending question: {}", question));
            }
        }
        ExecutionOutcome::NeedsUserInput { last_response, .. } => {
            console.info("AI is waiting for user input");
            if !last_response.is_empty() {
                console.info(&format!("Last response: {}", last_response));
            }
        }
    }

    // Always show key execution stats
    println!("ℹ Execution time: {:.2}s", duration.as_secs_f64());
    println!("ℹ Steps: {}", outcome.execution().steps.len());
    println!("ℹ Tokens: {}", outcome.execution().total_usage.total_tokens());

    // Show session file path if available
    if let Some(path) = session_path {
        println!("ℹ Session: {}", path.display());
    }

    // Show final result if available
    if let Some(final_result) = &outcome.execution().final_result {
        console.print_header("Final Result");
        println!("{}", final_result);
    }
}
