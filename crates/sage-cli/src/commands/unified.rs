//! Unified command implementation using the Claude Code style execution loop
//!
//! This module implements the new unified execution model where:
//! - There's no distinction between "run" and "interactive" modes
//! - User input blocks inline via InputChannel
//! - The execution loop never exits for user input

use crate::console::CLIConsole;
use crate::signal_handler::start_global_signal_handling;
use sage_core::agent::{ExecutionMode, ExecutionOptions, ExecutionOutcome, UnifiedExecutor};
use sage_core::config::{load_config_from_file, Config};
use sage_core::error::{SageError, SageResult};
use sage_core::input::{InputChannel, InputChannelHandle, InputResponse};
use sage_core::trajectory::TrajectoryRecorder;
use sage_core::types::TaskMetadata;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

/// Arguments for the unified command
pub struct UnifiedArgs {
    /// The task to execute (None for interactive mode with prompt)
    pub task: Option<String>,
    /// Path to configuration file
    pub config_file: String,
    /// Path to save trajectory file
    pub trajectory_file: Option<PathBuf>,
    /// Working directory for the agent
    pub working_dir: Option<PathBuf>,
    /// Maximum number of execution steps
    pub max_steps: Option<u32>,
    /// Enable verbose output
    pub verbose: bool,
    /// Non-interactive mode (auto-respond to questions)
    pub non_interactive: bool,
}

/// Execute a task using the unified execution loop
pub async fn execute(args: UnifiedArgs) -> SageResult<()> {
    let console = CLIConsole::new(args.verbose);

    // Initialize signal handling
    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    // Load configuration
    let config = if std::path::Path::new(&args.config_file).exists() {
        console.info(&format!("Loading configuration from: {}", args.config_file));
        load_config_from_file(&args.config_file)?
    } else {
        console.warn(&format!(
            "Configuration file not found: {}, using defaults",
            args.config_file
        ));
        Config::default()
    };

    // Get the task description
    let task_description = match args.task {
        Some(task) => {
            // Check if it's a file path
            if let Ok(task_path) = std::path::Path::new(&task).canonicalize() {
                if task_path.is_file() {
                    console.info(&format!("Loading task from file: {}", task_path.display()));
                    tokio::fs::read_to_string(&task_path)
                        .await
                        .map_err(|e| SageError::config(format!("Failed to read task file: {e}")))?
                } else {
                    task
                }
            } else {
                task
            }
        }
        None => {
            // Interactive mode - prompt for task
            console.print_header("Sage Agent - Unified Mode");
            console.info("Enter your task (Ctrl+D to finish):");
            let mut input = String::new();
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            while reader.read_line(&mut input).await? > 0 {}
            input.trim().to_string()
        }
    };

    if task_description.is_empty() {
        return Err(SageError::config("No task provided"));
    }

    // Set up execution options
    let mode = if args.non_interactive {
        ExecutionMode::non_interactive()
    } else {
        ExecutionMode::interactive()
    };

    let mut options = ExecutionOptions::default().with_mode(mode);

    if let Some(max_steps) = args.max_steps {
        options = options.with_max_steps(max_steps);
    }
    if let Some(working_dir) = &args.working_dir {
        options = options.with_working_directory(working_dir);
    }
    if let Some(trajectory_path) = &args.trajectory_file {
        options = options.with_trajectory_path(trajectory_path);
    }

    // Create the unified executor
    let mut executor = UnifiedExecutor::with_options(config.clone(), options)?;

    // Set up trajectory recording if requested
    if let Some(trajectory_path) = &args.trajectory_file {
        let recorder = TrajectoryRecorder::new(trajectory_path)?;
        executor.set_trajectory_recorder(Arc::new(Mutex::new(recorder)));
    }

    // Set up input channel for interactive mode
    let verbose = args.verbose;
    if !args.non_interactive {
        let (input_channel, input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);

        // Spawn task to handle user input
        tokio::spawn(handle_user_input(input_handle, verbose));
    }

    // Print task details
    console.print_header("Task Execution");
    console.info(&format!("Task: {}", task_description));
    console.info(&format!("Provider: {}", config.get_default_provider()));
    console.info(&format!("Max Steps: {}", executor.options().max_steps));
    console.print_separator();

    // Create task metadata
    let working_dir = args
        .working_dir
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });
    let task = TaskMetadata::new(&task_description, &working_dir);

    // Execute the task
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await?;
    let duration = start_time.elapsed();

    // Display results
    console.print_separator();
    display_outcome(&console, &outcome, duration);

    Ok(())
}

/// Handle user input requests from the execution loop
async fn handle_user_input(mut handle: InputChannelHandle, verbose: bool) {
    let console = CLIConsole::new(verbose);
    while let Some(request) = handle.request_rx.recv().await {
        // Display the question
        console.print_header("User Input Required");
        println!("{}", request.question);

        if let Some(options) = &request.options {
            for (idx, opt) in options.iter().enumerate() {
                println!("  {}. {}: {}", idx + 1, opt.label, opt.description);
            }
        }

        // Read user input using async stdin to avoid blocking the async runtime
        print!("> ");
        let _ = std::io::stdout().flush();

        // Use tokio's blocking task for stdin since std::io::stdin is blocking
        let input_result = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(_) => Some(input),
                Err(_) => None,
            }
        }).await;

        match input_result {
            Ok(Some(input)) => {
                let content = input.trim().to_string();

                // Check for cancel keywords
                let cancelled = content.to_lowercase() == "cancel"
                    || content.to_lowercase() == "quit"
                    || content.to_lowercase() == "exit";

                let response = if cancelled {
                    InputResponse::cancelled(request.id)
                } else {
                    InputResponse::text(request.id, content)
                };

                if let Err(e) = handle.respond(response).await {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
            }
            _ => {
                // EOF or error - send cancelled
                let _ = handle.respond(InputResponse::cancelled(request.id)).await;
                break;
            }
        }
    }
}

/// Display execution outcome
fn display_outcome(console: &CLIConsole, outcome: &ExecutionOutcome, duration: std::time::Duration) {
    match outcome {
        ExecutionOutcome::Success(_) => {
            console.success("Task completed successfully!");
        }
        ExecutionOutcome::Failed { error, .. } => {
            console.error("Task execution failed!");
            console.error(&format!("Error: {}", error.message));
            if let Some(suggestion) = &error.suggestion {
                console.info(&format!("Suggestion: {}", suggestion));
            }
        }
        ExecutionOutcome::Interrupted { .. } => {
            console.warn("Task interrupted by user (Ctrl+C)");
        }
        ExecutionOutcome::MaxStepsReached { .. } => {
            console.warn("Task reached maximum steps without completion");
            console.info("Consider breaking down the task or increasing max_steps");
        }
        ExecutionOutcome::UserCancelled { pending_question, .. } => {
            console.warn("Task cancelled by user");
            if let Some(question) = pending_question {
                console.info(&format!("Pending question: {}", question));
            }
        }
    }

    console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
    console.info(&format!(
        "Steps executed: {}",
        outcome.execution().steps.len()
    ));

    // Show token usage
    let usage = &outcome.execution().total_usage;
    console.info(&format!("Total tokens: {}", usage.total_tokens));

    // Show final result if available
    if let Some(final_result) = &outcome.execution().final_result {
        console.print_header("Final Result");
        println!("{}", final_result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_args_defaults() {
        let args = UnifiedArgs {
            task: None,
            config_file: "sage_config.json".to_string(),
            trajectory_file: None,
            working_dir: None,
            max_steps: None,
            verbose: false,
            non_interactive: false,
        };

        assert!(!args.non_interactive);
        assert!(!args.verbose);
    }
}
