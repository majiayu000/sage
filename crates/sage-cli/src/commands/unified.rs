//! Unified command implementation using the Claude Code style execution loop
//!
//! This module implements the new unified execution model where:
//! - There's no distinction between "run" and "interactive" modes
//! - User input blocks inline via InputChannel
//! - The execution loop never exits for user input

use crate::console::CliConsole;
use crate::signal_handler::start_global_signal_handling;
use sage_core::agent::{ExecutionMode, ExecutionOptions, ExecutionOutcome, UnifiedExecutor};
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::config::{Config, load_config_from_file};
use sage_core::error::{SageError, SageResult};
use sage_core::input::{InputChannel, InputChannelHandle, InputRequestKind, InputResponse};
use sage_core::trajectory::SessionRecorder;
use sage_core::types::TaskMetadata;
use sage_tools::get_default_tools;
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
    /// Working directory for the agent
    pub working_dir: Option<PathBuf>,
    /// Maximum number of execution steps
    pub max_steps: Option<u32>,
    /// Enable verbose output
    pub verbose: bool,
    /// Non-interactive mode (auto-respond to questions)
    pub non_interactive: bool,
    /// Resume a specific session by ID (for -r flag)
    pub resume_session_id: Option<String>,
    /// Resume the most recent session (for -c flag)
    pub continue_recent: bool,
    /// Stream JSON output mode (for SDK/programmatic use)
    pub stream_json: bool,
}

/// Execute a task using the unified execution loop
pub async fn execute(args: UnifiedArgs) -> SageResult<()> {
    let console = CliConsole::new(args.verbose);

    // Initialize signal handling
    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    // Load configuration
    let config = if std::path::Path::new(&args.config_file).exists() {
        load_config_from_file(&args.config_file)?
    } else {
        console.warn(&format!(
            "Configuration file not found: {}, using defaults",
            args.config_file
        ));
        Config::default()
    };

    // Determine working directory
    let working_dir = args
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Set up execution options
    let mode = if args.non_interactive {
        ExecutionMode::non_interactive()
    } else {
        ExecutionMode::interactive()
    };

    let mut options = ExecutionOptions::default().with_mode(mode);

    // Apply max_steps: if specified use it, otherwise keep unlimited (None)
    if let Some(max_steps) = args.max_steps {
        options = options.with_step_limit(max_steps);
    }
    options = options.with_working_directory(&working_dir);

    // Create the unified executor
    let mut executor = UnifiedExecutor::with_options(config.clone(), options)?;

    // Register default tools
    executor.register_tools(get_default_tools());

    // Initialize sub-agent support for Task tool
    if let Err(e) = executor.init_subagent_support() {
        console.warn(&format!("Failed to initialize sub-agent support: {}", e));
    }

    // Set up JSONL storage for session management
    let jsonl_storage = sage_core::session::JsonlSessionStorage::default_path()?;
    let jsonl_storage = Arc::new(jsonl_storage);
    executor.set_jsonl_storage(jsonl_storage.clone());

    // Enable JSONL session recording (Claude Code style)
    if let Err(e) = executor.enable_session_recording().await {
        console.warn(&format!("Failed to enable session recording: {}", e));
    }

    // Handle session resume (-c or -r flags)
    if args.continue_recent || args.resume_session_id.is_some() {
        return execute_session_resume(args, executor, console, config, working_dir).await;
    }

    // Handle stream JSON mode (for SDK/programmatic use)
    if args.stream_json {
        return execute_stream_json(args, executor, config, working_dir).await;
    }

    // Set up session recording - always enabled, stored in ~/.sage/projects/{cwd}/
    let session_recorder = if config.trajectory.is_enabled() {
        match SessionRecorder::new(&working_dir) {
            Ok(recorder) => {
                let recorder = Arc::new(Mutex::new(recorder));
                executor.set_session_recorder(recorder.clone());
                Some(recorder)
            }
            Err(e) => {
                console.warn(&format!("Failed to initialize session recorder: {}", e));
                None
            }
        }
    } else {
        None
    };

    // Set up input channel for interactive mode
    let verbose = args.verbose;
    if !args.non_interactive {
        let (input_channel, input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);
        tokio::spawn(handle_user_input(input_handle, verbose));
    }

    // Determine execution mode based on whether task was provided
    match args.task {
        Some(task) => {
            // One-shot mode: execute single task and exit
            let task_description = load_task_from_arg(&task, &console).await?;
            execute_single_task(
                &mut executor,
                &console,
                &config,
                &working_dir,
                &jsonl_storage,
                &session_recorder,
                &task_description,
            )
            .await
        }
        None => {
            // Interactive REPL mode: loop until user exits
            execute_interactive_loop(
                &mut executor,
                &console,
                &config,
                &working_dir,
                &jsonl_storage,
                &session_recorder,
            )
            .await
        }
    }
}

/// Load task description from argument (might be a file path)
async fn load_task_from_arg(task: &str, console: &CliConsole) -> SageResult<String> {
    if let Ok(task_path) = std::path::Path::new(task).canonicalize() {
        if task_path.is_file() {
            console.info(&format!("Loading task from file: {}", task_path.display()));
            return tokio::fs::read_to_string(&task_path)
                .await
                .map_err(|e| SageError::config(format!("Failed to read task file: {e}")));
        }
    }
    Ok(task.to_string())
}

/// Execute a single task (one-shot mode)
async fn execute_single_task(
    executor: &mut UnifiedExecutor,
    console: &CliConsole,
    _config: &Config,
    working_dir: &std::path::Path,
    jsonl_storage: &Arc<sage_core::session::JsonlSessionStorage>,
    session_recorder: &Option<Arc<Mutex<SessionRecorder>>>,
    task_description: &str,
) -> SageResult<()> {
    // Process slash commands if needed
    let task_description = process_slash_command(
        task_description,
        console,
        working_dir,
        jsonl_storage,
    )
    .await?;

    // If command was handled locally, we're done
    if task_description.is_none() {
        return Ok(());
    }
    let task_description = task_description.unwrap();

    // Execute the task
    let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await?;
    let duration = start_time.elapsed();

    // Display results
    console.print_separator();
    let session_path = if let Some(recorder) = session_recorder {
        Some(recorder.lock().await.file_path().to_path_buf())
    } else {
        None
    };
    display_outcome(console, &outcome, duration, session_path.as_ref());

    Ok(())
}

/// Interactive REPL loop (Claude Code style)
async fn execute_interactive_loop(
    executor: &mut UnifiedExecutor,
    console: &CliConsole,
    _config: &Config,
    working_dir: &std::path::Path,
    jsonl_storage: &Arc<sage_core::session::JsonlSessionStorage>,
    _session_recorder: &Option<Arc<Mutex<SessionRecorder>>>,
) -> SageResult<()> {
    // Show welcome header and recent activity
    console.print_header("Sage Agent");
    show_recent_activity(console, jsonl_storage).await;

    console.info("Type your message, or /help for commands. Press Ctrl+C to exit.");
    println!();

    // Main REPL loop - use console.input() for proper Chinese character handling
    loop {
        // Read user input with proper Unicode support (using console crate's Term)
        let input = match console.input("sage") {
            Ok(input) => input,
            Err(e) => {
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::Interrupted
                ) {
                    println!();
                    console.info("Goodbye!");
                    break;
                }
                console.error(&format!("Input error: {}", e));
                continue;
            }
        };

        // Skip empty input (console.input already handles this internally)
        if input.is_empty() {
            continue;
        }

        // Handle exit commands
        if input == "/exit" || input == "/quit" || input == "exit" || input == "quit" || input == "q" {
            console.info("Goodbye!");
            break;
        }

        // Handle /clear command
        if input == "/clear" || input == "clear" || input == "cls" {
            print!("\x1B[2J\x1B[1;1H");
            print!("\x1B[3J");
            console.success("Conversation cleared.");
            continue;
        }

        // Process slash commands
        let task_description = match process_slash_command(
            &input,
            console,
            working_dir,
            jsonl_storage,
        )
        .await
        {
            Ok(Some(desc)) => desc,
            Ok(None) => continue, // Command was handled locally
            Err(e) => {
                console.error(&format!("Command error: {}", e));
                continue;
            }
        };

        // Execute the task
        let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());
        let start_time = std::time::Instant::now();

        match executor.execute(task).await {
            Ok(outcome) => {
                let duration = start_time.elapsed();

                // Show brief stats (not full outcome display in REPL mode)
                println!();
                println!(
                    "\x1b[90m({:.1}s, {} tokens)\x1b[0m",
                    duration.as_secs_f64(),
                    outcome.execution().total_usage.total_tokens
                );
                println!();
            }
            Err(e) => {
                console.error(&format!("Execution error: {}", e));
            }
        }
    }

    Ok(())
}

/// Process slash commands, return None if handled locally, Some(prompt) if should be sent to LLM
async fn process_slash_command(
    input: &str,
    console: &CliConsole,
    working_dir: &std::path::Path,
    jsonl_storage: &Arc<sage_core::session::JsonlSessionStorage>,
) -> SageResult<Option<String>> {
    if !CommandExecutor::is_command(input) {
        return Ok(Some(input.to_string()));
    }

    let mut registry = CommandRegistry::new(working_dir);
    registry.register_builtins();
    if let Err(e) = registry.discover().await {
        console.warn(&format!("Failed to discover commands: {}", e));
    }

    let cmd_executor = CommandExecutor::new(Arc::new(tokio::sync::RwLock::new(registry)));

    match cmd_executor.process(input).await {
        Ok(Some(result)) => {
            // Handle interactive commands (e.g., /resume)
            if let Some(interactive_cmd) = result.interactive {
                handle_interactive_command(&interactive_cmd, console, jsonl_storage).await?;
                return Ok(None);
            }

            // Handle local commands (output directly, no LLM)
            if result.is_local {
                if let Some(status) = &result.status_message {
                    console.info(status);
                }
                if let Some(output) = &result.local_output {
                    println!("{}", output);
                }
                return Ok(None);
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
            Ok(Some(result.expanded_prompt))
        }
        Ok(None) => Ok(Some(input.to_string())), // Not a command, use as-is
        Err(e) => Err(e),
    }
}

/// Handle user input requests from the execution loop
async fn handle_user_input(mut handle: InputChannelHandle, verbose: bool) {
    let console = CliConsole::new(verbose);
    while let Some(request) = handle.request_rx.recv().await {
        // Display the question based on request kind
        console.print_header("User Input Required");

        match &request.kind {
            InputRequestKind::Questions { questions } => {
                for question in questions {
                    println!("{}", question.question);
                    for (idx, opt) in question.options.iter().enumerate() {
                        println!("  {}. {}: {}", idx + 1, opt.label, opt.description);
                    }
                }
            }
            InputRequestKind::Permission {
                tool_name,
                description,
                ..
            } => {
                println!("Permission required for tool: {}", tool_name);
                println!("{}", description);
                println!("Enter 'yes' or 'y' to allow, 'no' or 'n' to deny:");
            }
            InputRequestKind::FreeText { prompt, .. } => {
                println!("{}", prompt);
            }
            InputRequestKind::Simple {
                question, options, ..
            } => {
                println!("{}", question);
                if let Some(opts) = options {
                    for (idx, opt) in opts.iter().enumerate() {
                        println!("  {}. {}: {}", idx + 1, opt.label, opt.description);
                    }
                }
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
        })
        .await;

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
                    // Handle permission responses specially
                    if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                        let lower = content.to_lowercase();
                        if lower == "yes" || lower == "y" {
                            InputResponse::permission_granted(request.id)
                        } else if lower == "no" || lower == "n" {
                            InputResponse::permission_denied(
                                request.id,
                                Some("User denied".to_string()),
                            )
                        } else {
                            InputResponse::text(request.id, content)
                        }
                    } else {
                        InputResponse::text(request.id, content)
                    }
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
fn display_outcome(
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
    println!("ℹ Tokens: {}", outcome.execution().total_usage.total_tokens);

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

/// Execute session resume (-c or -r flags)
///
/// This function handles resuming a previous session, either the most recent one (-c)
/// or a specific session by ID (-r).
async fn execute_session_resume(
    args: UnifiedArgs,
    mut executor: UnifiedExecutor,
    console: CliConsole,
    config: Config,
    working_dir: PathBuf,
) -> SageResult<()> {
    // Determine which session to resume
    let session_id = if let Some(id) = args.resume_session_id {
        id
    } else {
        // Find most recent session for this working directory
        match executor.get_most_recent_session().await? {
            Some(metadata) => {
                console.info(&format!(
                    "Resuming most recent session: {} ({})",
                    metadata.id,
                    metadata.display_title()
                ));
                metadata.id
            }
            None => {
                return Err(SageError::config(
                    "No previous sessions found in this directory. Start a new session first.",
                ));
            }
        }
    };

    console.print_header("Session Resume");
    console.info(&format!("Resuming session: {}", session_id));

    // Restore the session (loads messages and sets up session state)
    let _restored_messages = executor.restore_session(&session_id).await?;

    console.success(&format!(
        "Session restored successfully"
    ));

    // Set up session recording
    let session_recorder = if config.trajectory.is_enabled() {
        match SessionRecorder::new(&working_dir) {
            Ok(recorder) => {
                let recorder = Arc::new(Mutex::new(recorder));
                executor.set_session_recorder(recorder.clone());
                Some(recorder)
            }
            Err(e) => {
                console.warn(&format!("Failed to initialize session recorder: {}", e));
                None
            }
        }
    } else {
        None
    };

    // Set up input channel for interactive mode
    let verbose = args.verbose;
    if !args.non_interactive {
        let (input_channel, input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);
        tokio::spawn(handle_user_input(input_handle, verbose));
    }

    // Print session info
    console.info(&format!("Provider: {}", config.get_default_provider()));
    let max_steps_display = match executor.options().max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!("Max Steps: {}", max_steps_display));
    console.print_separator();

    // Prompt for next user input
    console.info("Enter your next message to continue the conversation (Ctrl+D to finish):");
    let mut input = String::new();
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    while reader.read_line(&mut input).await? > 0 {}
    let next_message = input.trim().to_string();

    if next_message.is_empty() {
        console.info("No input provided. Session ready for future continuation.");
        return Ok(());
    }

    // Create task metadata with the new message
    let task = TaskMetadata::new(&next_message, &working_dir.display().to_string());

    // Execute the task (continuing from restored state)
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await?;
    let duration = start_time.elapsed();

    // Display results
    console.print_separator();

    let session_path = if let Some(recorder) = &session_recorder {
        Some(recorder.lock().await.file_path().to_path_buf())
    } else {
        None
    };

    display_outcome(&console, &outcome, duration, session_path.as_ref());

    Ok(())
}

/// Execute task with streaming JSON output (for SDK/programmatic use)
///
/// This function outputs events as JSON lines (JSONL format) for easy parsing
/// by other programs or SDKs. Each line is a complete JSON object.
///
/// Output format follows Claude Code compatible schema:
/// ```json
/// {"type":"system","message":"Starting...","timestamp":"..."}
/// {"type":"assistant","content":"I'll help you...","timestamp":"..."}
/// {"type":"tool_call_start","call_id":"...","tool_name":"Read","timestamp":"..."}
/// {"type":"tool_call_result","call_id":"...","tool_name":"Read","success":true,"timestamp":"..."}
/// {"type":"result","content":"Done","duration_ms":1234,"timestamp":"..."}
/// ```
async fn execute_stream_json(
    args: UnifiedArgs,
    mut executor: UnifiedExecutor,
    config: Config,
    working_dir: PathBuf,
) -> SageResult<()> {
    use sage_core::output::{OutputEvent, OutputFormat, OutputWriter, CostInfo};
    use std::io::stdout;

    // Create stream JSON writer
    let mut writer = OutputWriter::new(stdout(), OutputFormat::StreamJson);

    // Emit start event
    writer.write_event(&OutputEvent::system("Sage Agent starting")).ok();

    // Get task description - required for stream mode
    let task_description = match args.task {
        Some(task) => {
            // Check if it's a file path
            if let Ok(task_path) = std::path::Path::new(&task).canonicalize() {
                if task_path.is_file() {
                    writer.write_event(&OutputEvent::system(
                        &format!("Loading task from file: {}", task_path.display())
                    )).ok();
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
            writer.write_event(&OutputEvent::error("No task provided for stream mode")).ok();
            return Err(SageError::config("Stream JSON mode requires a task. Use: sage --stream-json \"your task\""));
        }
    };

    // Emit task received event
    writer.write_event(&OutputEvent::system(
        &format!("Task: {}", &task_description[..task_description.len().min(100)])
    )).ok();

    // Set up session recording
    let session_recorder = if config.trajectory.is_enabled() {
        match SessionRecorder::new(&working_dir) {
            Ok(recorder) => {
                let recorder = Arc::new(Mutex::new(recorder));
                executor.set_session_recorder(recorder.clone());
                Some(recorder)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // No input channel for stream mode (non-interactive by design)

    // Create task metadata
    let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());

    // Execute the task
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await;
    let duration = start_time.elapsed();

    // Get session ID if available
    let session_id = if let Some(recorder) = &session_recorder {
        Some(recorder.lock().await.session_id().to_string())
    } else {
        None
    };

    // Emit result based on outcome
    match outcome {
        Ok(ref execution_outcome) => {
            use sage_core::agent::ExecutionOutcome;

            // Build cost info from execution
            let execution = execution_outcome.execution();
            let mut cost = CostInfo::new(
                execution.total_usage.prompt_tokens as usize,
                execution.total_usage.completion_tokens as usize,
            );
            if let Some(cache_read) = execution.total_usage.cache_read_input_tokens {
                cost = cost.with_cache_read(cache_read as usize);
            }
            if let Some(cache_creation) = execution.total_usage.cache_creation_input_tokens {
                cost = cost.with_cache_creation(cache_creation as usize);
            }

            let result_content = match execution_outcome {
                ExecutionOutcome::Success(_) => {
                    execution.final_result.clone().unwrap_or_else(|| "Task completed successfully".to_string())
                }
                ExecutionOutcome::Failed { error, .. } => {
                    format!("Error: {}", error.message)
                }
                ExecutionOutcome::Interrupted { .. } => {
                    "Task interrupted by user".to_string()
                }
                ExecutionOutcome::MaxStepsReached { .. } => {
                    "Task reached maximum steps".to_string()
                }
                ExecutionOutcome::UserCancelled { .. } => {
                    "Task cancelled by user".to_string()
                }
                ExecutionOutcome::NeedsUserInput { last_response, .. } => {
                    format!("Waiting for input: {}", last_response)
                }
            };

            let mut result_event = match OutputEvent::result(&result_content) {
                OutputEvent::Result(mut e) => {
                    e.duration_ms = duration.as_millis() as u64;
                    e.cost = Some(cost);
                    if let Some(id) = session_id {
                        e.session_id = Some(id);
                    }
                    OutputEvent::Result(e)
                }
                _ => unreachable!(),
            };

            writer.write_event(&result_event).ok();
        }
        Err(ref e) => {
            writer.write_event(&OutputEvent::error(&e.to_string())).ok();
        }
    }

    outcome.map(|_| ())
}

/// Show recent activity card on startup (Claude Code style)
///
/// Displays a compact list of recent sessions to help users quickly resume
/// their previous work. Shows up to 3 sessions with titles and timestamps.
async fn show_recent_activity(
    console: &CliConsole,
    storage: &Arc<sage_core::session::JsonlSessionStorage>,
) {
    // Load recent sessions
    let sessions = match storage.list_sessions().await {
        Ok(s) => s,
        Err(_) => return, // Silently skip if we can't load sessions
    };

    if sessions.is_empty() {
        return;
    }

    // Show "Recent activity" header
    println!();
    println!("  \x1b[1mRecent activity\x1b[0m");
    println!();

    // Show up to 3 recent sessions
    let display_count = 3.min(sessions.len());
    for session in sessions.iter().take(display_count) {
        let title = session.display_title();
        let title_display = truncate_str(title, 45);

        let time_ago = format_time_ago(&session.updated_at);

        println!(
            "  \x1b[36m•\x1b[0m {} \x1b[90m({})\x1b[0m",
            title_display, time_ago
        );
    }

    // Show hint for more sessions
    if sessions.len() > display_count {
        println!();
        console.info(&format!(
            "{} more sessions. Use /resume to see all.",
            sessions.len() - display_count
        ));
    }

    println!();
}

/// Format time difference as human-readable string
fn format_time_ago(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_minutes() < 1 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{} min ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

/// Truncate a string to a maximum number of characters (UTF-8 safe)
fn truncate_str(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_chars {
        let truncated: String = chars[..max_chars.saturating_sub(3)].iter().collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Handle interactive commands that need CLI-level processing
async fn handle_interactive_command(
    cmd: &sage_core::commands::types::InteractiveCommand,
    console: &CliConsole,
    storage: &Arc<sage_core::session::JsonlSessionStorage>,
) -> SageResult<()> {
    use sage_core::commands::types::InteractiveCommand;

    match cmd {
        InteractiveCommand::Resume { session_id, show_all } => {
            handle_resume_command(session_id.as_deref(), *show_all, console, storage).await
        }
        InteractiveCommand::Title { title } => {
            console.warn(&format!("Title command not available in non-interactive mode. Title: {}", title));
            Ok(())
        }
    }
}

/// Handle /resume command - show and select sessions to resume
async fn handle_resume_command(
    session_id: Option<&str>,
    show_all: bool,
    console: &CliConsole,
    storage: &Arc<sage_core::session::JsonlSessionStorage>,
) -> SageResult<()> {
    let sessions = storage.list_sessions().await?;

    if sessions.is_empty() {
        console.info("No previous sessions found.");
        console.info("Start a conversation to create a new session.");
        return Ok(());
    }

    // If a specific session ID was provided, show info about resuming it
    if let Some(id) = session_id {
        // Find the session
        if let Some(session) = sessions.iter().find(|s| s.id == id || s.id.starts_with(id)) {
            console.print_header("Resume Session");
            println!();
            println!("  Session:  {}", session.id);
            println!("  Title:    {}", session.display_title());
            println!("  Modified: {}", session.updated_at.format("%Y-%m-%d %H:%M"));
            println!("  Messages: {}", session.message_count);
            println!();
            console.info(&format!("To resume this session, run: sage -r {}", session.id));
            return Ok(());
        } else {
            console.warn(&format!("Session not found: {}", id));
            console.info("Use /resume to see available sessions.");
            return Ok(());
        }
    }

    // Show list of sessions
    console.print_header("Recent Sessions");
    println!();

    let display_count = if show_all { sessions.len() } else { 10.min(sessions.len()) };

    for (i, session) in sessions.iter().take(display_count).enumerate() {
        let time_ago = format_time_ago(&session.updated_at);
        let title = session.display_title();
        let title_truncated = truncate_str(title, 50);

        println!(
            "  {}. {} ({}, {} msgs)",
            i + 1,
            title_truncated,
            time_ago,
            session.message_count
        );
        println!("     ID: {}", &session.id[..session.id.len().min(16)]);
        println!();
    }

    if !show_all && sessions.len() > display_count {
        console.info(&format!(
            "Showing {} of {} sessions. Use /resume --all to see all.",
            display_count,
            sessions.len()
        ));
    }

    println!();
    console.info("To resume a session:");
    console.info("  • Run: sage -r <session-id>");
    console.info("  • Or:  sage -c  (continue most recent)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_args_defaults() {
        let args = UnifiedArgs {
            task: None,
            config_file: "sage_config.json".to_string(),
            working_dir: None,
            max_steps: None,
            verbose: false,
            non_interactive: false,
            resume_session_id: None,
            continue_recent: false,
            stream_json: false,
        };

        assert!(!args.non_interactive);
        assert!(!args.verbose);
        assert!(!args.continue_recent);
        assert!(!args.stream_json);
    }
}
