//! Run command execution logic

use super::result_display::{display_result, display_token_usage};
use super::types::RunArgs;
use crate::console::CliConsole;
use crate::signal_handler::start_global_signal_handling;
use sage_core::commands::types::InteractiveCommand;
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::error::{SageError, SageResult};
use sage_sdk::{RunOptions, SageAgentSdk};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Execute the run command
pub async fn execute(args: RunArgs) -> SageResult<()> {
    let console = CliConsole::new(args.verbose);

    // Initialize signal handling for task interruption
    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    // Load and process task description
    let task_description = load_task_description(&args, &console).await?;

    // Process slash commands if the task starts with /
    let task_description = match process_slash_commands(&args, &task_description, &console).await? {
        SlashCommandResult::Continue(prompt) => prompt,
        SlashCommandResult::Completed => return Ok(()),
    };

    // Create SDK instance with configuration
    let sdk = create_sdk_instance(&args, &console)?;

    // Print task details
    print_task_info(&sdk, &task_description, &console);

    // Set up run options
    let run_options = build_run_options(&args);

    // Execute the task
    execute_task(&sdk, &task_description, run_options, &args, &console).await
}

/// Load task description from file or use directly
async fn load_task_description(args: &RunArgs, console: &CliConsole) -> SageResult<String> {
    if let Ok(task_path) = std::path::Path::new(&args.task).canonicalize() {
        if task_path.is_file() {
            console.info(&format!("Loading task from file: {}", task_path.display()));
            return tokio::fs::read_to_string(&task_path)
                .await
                .map_err(|e| SageError::config(format!("Failed to read task file: {e}")));
        }
    }
    Ok(args.task.clone())
}

/// Result of processing slash commands
pub enum SlashCommandResult {
    /// Continue with the expanded prompt
    Continue(String),
    /// Command completed, exit early
    Completed,
}

/// Process slash commands in task description
async fn process_slash_commands(
    args: &RunArgs,
    task_description: &str,
    console: &CliConsole,
) -> SageResult<SlashCommandResult> {
    if !CommandExecutor::is_command(task_description) {
        return Ok(SlashCommandResult::Continue(task_description.to_string()));
    }

    let working_dir = args
        .working_dir
        .as_ref()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mut registry = CommandRegistry::new(&working_dir);
    registry.register_builtins();
    if let Err(e) = registry.discover().await {
        console.warn(&format!("Failed to discover commands: {}", e));
    }

    let cmd_executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));

    match cmd_executor.process(task_description).await {
        Ok(Some(result)) => {
            // Handle interactive commands
            if let Some(interactive_cmd) = &result.interactive {
                handle_interactive_command(interactive_cmd, console)?;
                return Ok(SlashCommandResult::Completed);
            }

            // Handle local commands - display directly and return
            if result.is_local {
                if let Some(status) = &result.status_message {
                    console.info(status);
                }
                if let Some(output) = &result.local_output {
                    println!("{}", output);
                }
                return Ok(SlashCommandResult::Completed);
            }

            if result.show_expansion {
                console.info(&format!(
                    "Command expanded: {}",
                    &result.expanded_prompt[..result.expanded_prompt.len().min(100)]
                ));
            }
            if let Some(status) = &result.status_message {
                console.info(status);
            }
            Ok(SlashCommandResult::Continue(result.expanded_prompt))
        }
        Ok(None) => Ok(SlashCommandResult::Continue(task_description.to_string())),
        Err(e) => {
            console.error(&format!("Command error: {}", e));
            Err(e)
        }
    }
}

/// Create SDK instance with configuration
fn create_sdk_instance(args: &RunArgs, console: &CliConsole) -> SageResult<SageAgentSdk> {
    // Create SDK instance
    let mut sdk = if std::path::Path::new(&args.config_file).exists() {
        console.info(&format!("Loading configuration from: {}", args.config_file));
        SageAgentSdk::with_config_file(&args.config_file)?
    } else {
        console.warn(&format!(
            "Configuration file not found: {}, using defaults",
            args.config_file
        ));
        SageAgentSdk::new()?
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
        sdk = sdk.with_step_limit(max_steps);
    }

    if let Some(trajectory_file) = &args.trajectory_file {
        sdk = sdk.with_trajectory_path(trajectory_file);
    }

    // Validate configuration
    console.info("Validating configuration...");
    sdk.validate_config()?;

    Ok(sdk)
}

/// Print task information
fn print_task_info(sdk: &SageAgentSdk, task_description: &str, console: &CliConsole) {
    console.print_header("Task Execution");
    console.info(&format!("Task: {}", task_description));
    console.info(&format!("Provider: {}", sdk.config().default_provider));

    if let Ok(params) = sdk.config().default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }

    let max_steps_display = match sdk.config().max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!("Max Steps: {}", max_steps_display));

    if let Some(working_dir) = &sdk.config().working_directory {
        console.info(&format!("Working Directory: {}", working_dir.display()));
    }
}

/// Build run options from arguments
fn build_run_options(args: &RunArgs) -> RunOptions {
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

    run_options
}

/// Execute the task and display results
async fn execute_task(
    sdk: &SageAgentSdk,
    task_description: &str,
    run_options: RunOptions,
    args: &RunArgs,
    console: &CliConsole,
) -> SageResult<()> {
    console.print_separator();
    console.info("Starting task execution...");

    let start_time = std::time::Instant::now();

    match sdk.run_with_options(task_description, run_options).await {
        Ok(result) => {
            let duration = start_time.elapsed();

            console.print_separator();

            // Display result
            display_result(&result, console, args.verbose);

            // Display execution metrics
            console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
            console.info(&format!(
                "Steps executed: {}",
                result.execution().steps.len()
            ));

            // Display token usage
            display_token_usage(&result, console);

            // Handle patch creation
            if args.must_patch || args.patch_path.is_some() {
                console.warn("Patch creation feature not yet implemented");
                console.info(
                    "Files were modified during execution but patch generation is not available",
                );
            }

            Ok(())
        }
        Err(e) => {
            let duration = start_time.elapsed();
            console.print_separator();

            display_system_error(&e, sdk, console);

            console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
            Err(e)
        }
    }
}

/// Display system-level errors
fn display_system_error(e: &SageError, sdk: &SageAgentSdk, console: &CliConsole) {
    console.error("System error!");
    console.print_header("Error Details");
    console.error(&format!("Error: {}", e));

    // Print additional error context if available
    match e {
        SageError::Tool {
            tool_name, message, ..
        } => {
            console.info(&format!("Tool: {}", tool_name));
            console.info(&format!("Error: {}", message));
        }
        SageError::Llm { message: msg, .. } => {
            console.info(&format!("LLM Provider: {}", sdk.config().default_provider));
            console.info(&format!("Error: {}", msg));
        }
        SageError::Config { message: msg, .. } => {
            console.info(&format!("Configuration Error: {}", msg));
        }
        _ => {}
    }
}

/// Handle interactive commands in run mode
fn handle_interactive_command(
    cmd: &InteractiveCommand,
    console: &CliConsole,
) -> SageResult<()> {
    match cmd {
        InteractiveCommand::Resume { .. } => {
            console.warn("Session resume is not available in run mode.");
            console.info("Use `sage -c` or `sage -r <session_id>` instead.");
            Ok(())
        }
        InteractiveCommand::Title { .. } => {
            console.warn("The /title command is only available in interactive mode.");
            Ok(())
        }
    }
}
