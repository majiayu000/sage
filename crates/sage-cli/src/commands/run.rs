//! Run command implementation

use crate::console::CLIConsole;
use crate::signal_handler::start_global_signal_handling;
use std::collections::HashMap;
use std::path::PathBuf;
use sage_core::error::{SageError, SageResult};
use sage_sdk::{RunOptions, SageAgentSDK, ExecutionOutcome, ExecutionErrorKind};

/// Arguments for the run command
pub struct RunArgs {
    pub task: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub model_base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_steps: Option<u32>,
    pub working_dir: Option<PathBuf>,
    pub config_file: String,
    pub trajectory_file: Option<PathBuf>,
    pub patch_path: Option<PathBuf>,
    pub must_patch: bool,
    pub verbose: bool,
}

/// Execute the run command
pub async fn execute(args: RunArgs) -> SageResult<()> {
    let console = CLIConsole::new(args.verbose);

    // Initialize signal handling for task interruption
    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    // Load task from file if it's a file path
    let task_description = if let Ok(task_path) = std::path::Path::new(&args.task).canonicalize() {
        if task_path.is_file() {
            console.info(&format!("Loading task from file: {}", task_path.display()));
            tokio::fs::read_to_string(&task_path)
                .await
                .map_err(|e| SageError::config(format!("Failed to read task file: {e}")))?
        } else {
            args.task
        }
    } else {
        args.task
    };

    // Create command line overrides
    let mut overrides = HashMap::new();
    if let Some(provider) = &args.provider {
        overrides.insert("provider".to_string(), provider.clone());
    }
    if let Some(model) = &args.model {
        overrides.insert("model".to_string(), model.clone());
    }
    if let Some(api_key) = &args.api_key {
        overrides.insert("api_key".to_string(), api_key.clone());
    }
    if let Some(base_url) = &args.model_base_url {
        overrides.insert("model_base_url".to_string(), base_url.clone());
    }
    if let Some(max_steps) = args.max_steps {
        overrides.insert("max_steps".to_string(), max_steps.to_string());
    }
    if let Some(working_dir) = &args.working_dir {
        overrides.insert("working_dir".to_string(), working_dir.to_string_lossy().to_string());
    }

    // Create SDK instance
    let mut sdk = if std::path::Path::new(&args.config_file).exists() {
        console.info(&format!("Loading configuration from: {}", args.config_file));
        SageAgentSDK::with_config_file(&args.config_file)?
    } else {
        console.warn(&format!("Configuration file not found: {}, using defaults", args.config_file));
        SageAgentSDK::new()?
    };

    // Apply command line overrides
    if let Some(provider) = &args.provider {
        let model = args.model.as_deref().unwrap_or("gpt-4");
        sdk = sdk.with_provider_and_model(provider, model, args.api_key.as_deref())?;
    }

    if let Some(working_dir) = &args.working_dir {
        sdk = sdk.with_working_directory(working_dir);
    }

    if let Some(max_steps) = args.max_steps {
        sdk = sdk.with_max_steps(max_steps);
    }

    if let Some(trajectory_file) = &args.trajectory_file {
        sdk = sdk.with_trajectory_path(trajectory_file);
    }

    // Validate configuration
    console.info("Validating configuration...");
    sdk.validate_config()?;

    // Print task details
    console.print_header("Task Execution");
    console.info(&format!("Task: {}", task_description));
    console.info(&format!("Provider: {}", sdk.config().default_provider));
    
    if let Ok(params) = sdk.config().default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }
    
    console.info(&format!("Max Steps: {}", sdk.config().max_steps));
    
    if let Some(working_dir) = &sdk.config().working_directory {
        console.info(&format!("Working Directory: {}", working_dir.display()));
    }

    // Set up run options
    let mut run_options = RunOptions::new();
    
    if let Some(working_dir) = &args.working_dir {
        run_options = run_options.with_working_directory(working_dir);
    }
    
    if let Some(max_steps) = args.max_steps {
        run_options = run_options.with_max_steps(max_steps);
    }
    
    if args.trajectory_file.is_some() {
        run_options = run_options.with_trajectory(true);
        if let Some(path) = &args.trajectory_file {
            run_options = run_options.with_trajectory_path(path);
        }
    }

    // Execute the task
    console.print_separator();
    console.info("Starting task execution...");
    
    let start_time = std::time::Instant::now();
    
    match sdk.run_with_options(&task_description, run_options).await {
        Ok(result) => {
            let duration = start_time.elapsed();

            console.print_separator();

            // Handle different execution outcomes with detailed messages
            match &result.outcome {
                ExecutionOutcome::Success(_) => {
                    console.success("Task completed successfully!");
                }
                ExecutionOutcome::Failed { error, .. } => {
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
                ExecutionOutcome::Interrupted { .. } => {
                    console.warn("🛑 Task interrupted by user (Ctrl+C)");
                    console.info("Task was stopped gracefully.");
                }
                ExecutionOutcome::MaxStepsReached { .. } => {
                    console.warn("⚠ Task reached maximum steps without completion");
                    console.info("Consider breaking down the task or increasing max_steps");
                }
            }

            console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
            console.info(&format!("Steps executed: {}", result.execution().steps.len()));
            console.info(&format!("Total tokens: {}", result.execution().total_usage.total_tokens));

            if let Some(final_result) = result.final_result() {
                console.print_header("Final Result");
                println!("{}", final_result);
            }

            if let Some(trajectory_path) = result.trajectory_path() {
                console.info(&format!("Trajectory saved to: {}", trajectory_path.display()));
            }

            // Handle patch creation
            if args.must_patch || args.patch_path.is_some() {
                console.info("Creating patch...");
                // TODO: Implement patch creation
                console.warn("Patch creation not yet implemented in Rust version");
            }

            // Print statistics if verbose
            if args.verbose {
                console.print_header("Execution Statistics");
                let stats = result.statistics();
                console.info(&format!("Successful steps: {}", stats.successful_steps));
                console.info(&format!("Failed steps: {}", stats.failed_steps));
                console.info(&format!("Tool calls: {}", stats.tool_calls));

                if !stats.tool_usage.is_empty() {
                    console.info("Tool usage:");
                    for (tool, count) in &stats.tool_usage {
                        console.info(&format!("  {}: {} times", tool, count));
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            let duration = start_time.elapsed();
            console.print_separator();

            // This branch now only handles system-level errors
            // (e.g., couldn't create agent, couldn't connect to API at all)
            console.error("System error!");
            console.print_header("Error Details");
            console.error(&format!("Error: {}", e));

            // Print additional error context if available
            match &e {
                SageError::Tool { tool_name, message } => {
                    console.info(&format!("Tool: {}", tool_name));
                    console.info(&format!("Error: {}", message));
                }
                SageError::Llm(msg) => {
                    console.info(&format!("LLM Provider: {}", sdk.config().default_provider));
                    console.info(&format!("Error: {}", msg));
                }
                SageError::Config(msg) => {
                    console.info(&format!("Configuration Error: {}", msg));
                }
                _ => {}
            }

            console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
            Err(e)
        }
    }
}
