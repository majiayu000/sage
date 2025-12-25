//! Run command implementation

use crate::commands::session_resume::{SessionSelector, print_session_details};
use crate::console::CliConsole;
use crate::signal_handler::start_global_signal_handling;
use sage_core::commands::types::InteractiveCommand;
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::error::{SageError, SageResult};
use sage_sdk::{ExecutionErrorKind, ExecutionOutcome, RunOptions, SageAgentSdk};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    let console = CliConsole::new(args.verbose);

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

    // Process slash commands if the task starts with /
    let task_description = if CommandExecutor::is_command(&task_description) {
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

        match cmd_executor.process(&task_description).await {
            Ok(Some(result)) => {
                // Handle interactive commands
                if let Some(interactive_cmd) = &result.interactive {
                    return handle_interactive_command(interactive_cmd, &console).await;
                }

                // Handle local commands - display directly and return
                if result.is_local {
                    if let Some(status) = &result.status_message {
                        console.info(status);
                    }
                    if let Some(output) = &result.local_output {
                        println!("{}", output);
                    }
                    return Ok(());
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
                result.expanded_prompt
            }
            Ok(None) => task_description, // Not a command, use as-is
            Err(e) => {
                console.error(&format!("Command error: {}", e));
                return Err(e);
            }
        }
    } else {
        task_description
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
        overrides.insert(
            "working_dir".to_string(),
            working_dir.to_string_lossy().to_string(),
        );
    }

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

    // Print task details
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

            console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
            console.info(&format!(
                "Steps executed: {}",
                result.execution().steps.len()
            ));

            // Show token usage with cache info
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

            if let Some(final_result) = result.final_result() {
                console.print_header("Final Result");
                println!("{}", final_result);
            }

            if let Some(trajectory_path) = result.trajectory_path() {
                console.info(&format!(
                    "Trajectory saved to: {}",
                    trajectory_path.display()
                ));
            }

            // Handle patch creation
            // Patch creation requires:
            // 1. Detecting all file modifications made during execution (track via file tool results)
            // 2. Generating unified diff format for each modified file
            // 3. Combining diffs into a single patch file
            // 4. Writing patch to specified path or auto-generated location
            // This feature is planned but not yet implemented
            if args.must_patch || args.patch_path.is_some() {
                console.warn("Patch creation feature not yet implemented");
                console.info(
                    "Files were modified during execution but patch generation is not available",
                );
            }

            // Print statistics if verbose
            if args.verbose {
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

            console.info(&format!("Execution time: {:.2}s", duration.as_secs_f64()));
            Err(e)
        }
    }
}

/// Handle interactive commands that require CLI interaction
async fn handle_interactive_command(
    cmd: &InteractiveCommand,
    console: &CliConsole,
) -> SageResult<()> {
    match cmd {
        InteractiveCommand::Resume {
            session_id,
            show_all,
        } => handle_resume_command(session_id.clone(), *show_all, console).await,
    }
}

/// Handle the /resume command with interactive session selection
async fn handle_resume_command(
    session_id: Option<String>,
    show_all: bool,
    console: &CliConsole,
) -> SageResult<()> {
    let selector = SessionSelector::new()?.show_all_projects(show_all);

    // If session ID is provided, resume directly
    if let Some(id) = session_id {
        // Filter out --all flag from session ID
        let clean_id = if id == "--all" || id == "-a" {
            None
        } else {
            Some(id)
        };

        if let Some(id) = clean_id {
            match selector.resume_by_id(&id).await? {
                Some(result) => {
                    console.success(&format!("Resuming session: {}", result.session_id));
                    print_session_details(&result.metadata);

                    // Load and display recent messages
                    let messages = selector.storage().load_messages(&result.session_id).await?;
                    if !messages.is_empty() {
                        console.info(&format!(
                            "Session has {} messages. Ready to continue.",
                            messages.len()
                        ));

                        // Show last few messages as context
                        let recent: Vec<_> = messages.iter().rev().take(3).collect();
                        if !recent.is_empty() {
                            console.info("Recent conversation:");
                            for msg in recent.iter().rev() {
                                let role = if msg.is_user() {
                                    "You"
                                } else if msg.is_assistant() {
                                    "Assistant"
                                } else {
                                    "System"
                                };
                                let content = &msg.message.content;
                                let content_preview = if content.len() > 100 {
                                    format!("{}...", &content[..100])
                                } else {
                                    content.clone()
                                };
                                console.info(&format!("  {}: {}", role, content_preview));
                            }
                        }
                    }

                    // Session resumption requires loading previous messages into agent context:
                    // 1. Load all messages from the session storage
                    // 2. Create or extend the agent's message history with loaded messages
                    // 3. Pass the extended history to the SDK's run method
                    // 4. Ensure tool results and context are properly reconstructed
                    // This feature requires SDK changes to support pre-loading conversation history
                    console.info("\nTo continue this session, run:");
                    console.info(&format!(
                        "  sage run --session {} \"<your message>\"",
                        result.session_id
                    ));

                    return Ok(());
                }
                None => {
                    return Err(SageError::not_found(format!("Session '{}' not found", id)));
                }
            }
        }
    }

    // Interactive session selection
    match selector.select_session().await? {
        Some(result) => {
            console.success(&format!("Selected session: {}", result.session_id));
            print_session_details(&result.metadata);

            // Check if we need to change directory
            let current_dir = std::env::current_dir().unwrap_or_default();
            if result.working_directory != current_dir {
                console.warn("This session is from a different directory.");
                console.info(&format!(
                    "Session directory: {}",
                    result.working_directory.display()
                ));
                console.info(&format!("Current directory: {}", current_dir.display()));
                console.info("\nTo resume this session, run:");
                console.info(&format!(
                    "  cd {} && sage run --session {} \"<your message>\"",
                    result.working_directory.display(),
                    result.session_id
                ));
            } else {
                console.info("\nTo continue this session, run:");
                console.info(&format!(
                    "  sage run --session {} \"<your message>\"",
                    result.session_id
                ));
            }

            Ok(())
        }
        None => {
            // User cancelled or no sessions
            Ok(())
        }
    }
}
