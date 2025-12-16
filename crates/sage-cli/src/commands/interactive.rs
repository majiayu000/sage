//! Interactive mode implementation

use crate::console::CLIConsole;
use crate::signal_handler::{AppState, set_global_app_state, start_global_signal_handling};
use sage_core::agent::AgentExecution;
use sage_core::error::{SageError, SageResult};
use sage_core::llm::messages::LLMMessage;
use sage_core::types::TaskMetadata;
use sage_core::ui::EnhancedConsole;
use sage_sdk::{ExecutionErrorKind, ExecutionOutcome, RunOptions, SageAgentSDK};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

/// Conversation session manager for interactive mode
struct ConversationSession {
    /// Current conversation messages
    messages: Vec<LLMMessage>,
    /// Current task metadata
    task: Option<TaskMetadata>,
    /// Current agent execution
    execution: Option<AgentExecution>,
    /// Session metadata
    metadata: HashMap<String, serde_json::Value>,
    /// Whether this is the first message in the conversation
    is_first_message: bool,
}

impl ConversationSession {
    /// Create a new conversation session
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            task: None,
            execution: None,
            metadata: HashMap::new(),
            is_first_message: true,
        }
    }

    /// Add a user message to the conversation
    fn add_user_message(&mut self, content: &str) {
        self.messages.push(LLMMessage::user(content));
    }

    /// Add an assistant message to the conversation
    fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(LLMMessage::assistant(content));
    }

    /// Check if this is a new conversation (no messages yet)
    fn is_new_conversation(&self) -> bool {
        self.is_first_message
    }

    /// Mark that the first message has been processed
    fn mark_first_message_processed(&mut self) {
        self.is_first_message = false;
    }

    /// Reset the conversation session
    fn reset(&mut self) {
        self.messages.clear();
        self.task = None;
        self.execution = None;
        self.metadata.clear();
        self.is_first_message = true;
    }

    /// Get conversation summary
    fn get_summary(&self) -> String {
        format!("Conversation with {} messages", self.messages.len())
    }
}

/// Arguments for interactive mode
pub struct InteractiveArgs {
    pub config_file: String,
    pub trajectory_file: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
}

/// Execute interactive mode
pub async fn execute(args: InteractiveArgs) -> SageResult<()> {
    let console = CLIConsole::new(true);

    // Initialize signal handling for task interruption
    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    // Use enhanced console for beautiful welcome
    EnhancedConsole::print_welcome_banner();
    EnhancedConsole::print_section_header(
        "Interactive Mode",
        Some("Type 'help' for available commands, 'exit' to quit"),
    );

    // Initialize SDK
    let mut sdk = if std::path::Path::new(&args.config_file).exists() {
        console.info(&format!("Loading configuration from: {}", args.config_file));
        SageAgentSDK::with_config_file(&args.config_file)?
    } else {
        console.warn(&format!(
            "Configuration file not found: {}, using defaults",
            args.config_file
        ));
        SageAgentSDK::new()?
    };

    if let Some(working_dir) = &args.working_dir {
        sdk = sdk.with_working_directory(working_dir);
    }

    if let Some(trajectory_file) = &args.trajectory_file {
        sdk = sdk.with_trajectory_path(trajectory_file);
    }

    console.success("Interactive mode initialized");
    console.print_separator();

    // Initialize conversation session
    let mut conversation = ConversationSession::new();

    // Main interactive loop
    loop {
        // Ensure we're in a clean state before each iteration
        std::io::stdout().flush().unwrap_or(());
        std::io::stderr().flush().unwrap_or(());

        // Set state to waiting for input
        set_global_app_state(AppState::WaitingForInput);

        match console.input("sage") {
            Ok(input) => {
                let input = input.trim();

                // Skip empty input (including backspace artifacts)
                if input.is_empty() {
                    continue;
                }

                // Check for common backspace artifacts
                if input.chars().all(|c| c.is_whitespace() || c.is_control()) {
                    console.warn("检测到输入异常，已清理。请重新输入：");
                    continue;
                }

                match input {
                    "exit" | "quit" | "q" => {
                        console.info("Goodbye!");
                        return Ok(());
                    }
                    "help" | "h" => {
                        print_help(&console);
                    }
                    "config" => {
                        print_config(&console, &sdk);
                    }
                    "status" => {
                        print_status(&console, &sdk);
                    }
                    "clear" | "cls" => {
                        // Clear screen and reset display
                        print!("\x1B[2J\x1B[1;1H"); // Clear screen
                        print!("\x1B[3J"); // Clear scrollback buffer
                        console.success("Screen cleared!");
                    }
                    "reset" | "refresh" => {
                        // Force terminal reset to fix display issues
                        print!("\r\x1B[K"); // Clear current line
                        print!("\x1B[2J\x1B[1;1H"); // Clear screen
                        print!("\x1B[3J"); // Clear scrollback
                        console.success("Terminal display reset!");
                    }
                    "input-help" | "ih" => {
                        print_input_help(&console);
                    }
                    "new" | "new-task" => {
                        // Start a new conversation/task
                        conversation.reset();
                        console.success("Started new conversation. Previous context cleared.");
                    }
                    "conversation" | "conv" => {
                        // Show conversation summary
                        console.info(&format!(
                            "Current conversation: {}",
                            conversation.get_summary()
                        ));
                    }
                    _ => {
                        // Set state to executing task
                        set_global_app_state(AppState::ExecutingTask);

                        // Handle conversation mode
                        match handle_conversation(&console, &sdk, &mut conversation, input).await {
                            Ok(()) => {
                                // Conversation handled successfully
                            }
                            Err(e) => {
                                console.error(&format!("Conversation failed: {e}"));

                                // Check if this is a critical error that should break the loop
                                if is_critical_error(&e) {
                                    console.error(
                                        "Critical error encountered. Exiting interactive mode.",
                                    );
                                    break;
                                }

                                // For non-critical errors, continue the loop
                                console.info(
                                    "You can try again or type 'help' for available commands.",
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // Check if this is EOF or Ctrl+C interruption
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    // EOF detected - exit without message (signal handler will handle it)
                    break;
                } else if e.kind() == std::io::ErrorKind::Interrupted {
                    // User pressed Ctrl+C during input prompt - exit without message
                    // The signal handler will print the goodbye message
                    break;
                } else {
                    console.error(&format!("Input error: {e}"));
                    // For other input errors, try to continue
                    console.warn("Input error occurred. Please try again.");
                    continue;
                }
            }
        }

        console.print_separator();
    }

    // If we reach here, it means we exited due to an error or interruption
    // The goodbye message is handled elsewhere (signal handler or explicit exit command)
    Ok(())
}

/// Check if an error is critical and should terminate the interactive session
fn is_critical_error(error: &SageError) -> bool {
    match error {
        // Configuration errors are critical
        SageError::Config(_) => true,
        // LLM client errors might be temporary, so not critical
        SageError::Llm(_) => false,
        // Tool errors are usually not critical
        SageError::Tool { .. } => false,
        // Agent errors might be critical
        SageError::Agent(_) => false,
        // IO errors might be critical depending on the context
        SageError::Io(_) => false,
        // JSON errors are usually not critical
        SageError::Json(_) => false,
        // HTTP errors are usually temporary
        SageError::Http(_) => false,
        // Invalid input is not critical
        SageError::InvalidInput(_) => false,
        // Timeout is not critical
        SageError::Timeout { .. } => false,
        // Cancelled is not critical
        SageError::Cancelled => false,
        // Other errors are generally not critical
        _ => false,
    }
}

/// Print help information
fn print_help(console: &CLIConsole) {
    console.print_header("Available Commands");
    console.info("help, h          - Show this help message");
    console.info("config           - Show current configuration");
    console.info("status           - Show system status");
    console.info("clear, cls       - Clear the screen");
    console.info("reset, refresh   - Reset terminal display (fixes backspace issues)");
    console.info("input-help, ih   - Show input troubleshooting help");
    console.info("new, new-task    - Start a new conversation (clears previous context)");
    console.info("conversation, conv - Show current conversation summary");
    console.info("exit, quit, q    - Exit interactive mode");
    console.info("");
    console.info("🗣️  Conversation Mode:");
    console.info("Any other input will be treated as part of an ongoing conversation.");
    console
        .info("The AI will remember previous messages and context within the same conversation.");
    console.info("Use 'new' to start fresh if you want to change topics completely.");
    console.info("");
    console.info("Example conversation:");
    console.info("  You: Create a hello world Python script");
    console.info("  AI: [Creates the script]");
    console.info("  You: Now add error handling to it");
    console.info("  AI: [Modifies the existing script with error handling]");
}

/// Print input troubleshooting help
fn print_input_help(console: &CLIConsole) {
    console.print_header("退格键问题解决方案");

    console.info("如果遇到退格键删除后仍显示字符的问题：");
    console.info("");
    console.info("立即解决方案：");
    console.info("  reset          - 重置终端显示（推荐）");
    console.info("  clear          - 清屏重新开始");
    console.info("  Ctrl+U         - 清除当前行");
    console.info("");
    console.info("常见问题和解决方法：");
    console.info("  • 中文输入残留:    输入 'reset' 重置显示");
    console.info("  • 退格键异常:      切换到英文输入法");
    console.info("  • 字符显示错乱:    使用 Ctrl+U 清除整行");
    console.info("  • 输入法问题:      重启输入法或切换输入法");
    console.info("");
    console.info("预防措施：");
    console.info("  • 输入命令时使用英文输入法");
    console.info("  • 避免在输入过程中频繁切换输入法");
    console.info("  • 使用支持中文较好的终端（如 iTerm2）");
    console.info("");
    console.info("终端快捷键：");
    console.info("  • Ctrl+U         - 清除当前行");
    console.info("  • Ctrl+A         - 移动到行首");
    console.info("  • Ctrl+E         - 移动到行尾");
    console.info("  • Ctrl+C         - 取消当前输入");
}

/// Print current configuration
fn print_config(console: &CLIConsole, sdk: &SageAgentSDK) {
    console.print_header("Current Configuration");
    let config = sdk.config();

    console.info(&format!("Provider: {}", config.default_provider));

    if let Ok(params) = config.default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }

    console.info(&format!("Max Steps: {}", config.max_steps));

    if let Some(working_dir) = &config.working_directory {
        console.info(&format!("Working Directory: {}", working_dir.display()));
    }

    console.info(&format!(
        "Tools Enabled: {}",
        config.tools.enabled_tools.len()
    ));
}

/// Print system status
fn print_status(console: &CLIConsole, sdk: &SageAgentSDK) {
    console.print_header("Agent Status");

    let config = sdk.config();

    // Show key information like Python version
    console.info(&format!("Provider: {}", config.get_default_provider()));

    if let Ok(params) = config.default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }

    console.info(&format!(
        "Available Tools: {}",
        config.tools.enabled_tools.len()
    ));
    console.info(&format!("Max Steps: {}", config.max_steps));

    // Check configuration validity
    match sdk.validate_config() {
        Ok(()) => console.success("Configuration is valid"),
        Err(e) => console.error(&format!("Configuration error: {e}")),
    }

    // Check API keys
    for (provider, params) in &config.model_providers {
        let has_key = params.get_api_key().is_some();
        let status = if has_key { "✓" } else { "✗" };
        console.info(&format!(
            "{status} {provider}: API key {}",
            if has_key { "configured" } else { "missing" }
        ));
    }

    // Check working directory
    if let Some(working_dir) = &config.working_directory {
        if working_dir.exists() {
            console.success(&format!(
                "Working directory accessible: {}",
                working_dir.display()
            ));
        } else {
            console.error(&format!(
                "Working directory not found: {}",
                working_dir.display()
            ));
        }
    } else {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        console.info(&format!(
            "Using current directory: {}",
            current_dir.display()
        ));
    }
}

/// Handle conversation mode - supports continuous dialogue
async fn handle_conversation(
    console: &CLIConsole,
    sdk: &SageAgentSDK,
    conversation: &mut ConversationSession,
    user_input: &str,
) -> SageResult<()> {
    // Add user message to conversation
    conversation.add_user_message(user_input);

    if conversation.is_new_conversation() {
        // This is the first message, create a new task
        console.print_header("New Conversation");
        console.info(&format!("Message: {user_input}"));

        // Create task metadata for the conversation
        let working_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let task = TaskMetadata::new(user_input, &working_dir);
        conversation.task = Some(task.clone());

        // Execute the initial task
        execute_conversation_task(console, sdk, conversation, &task).await
    } else {
        // This is a continuation of existing conversation
        console.print_header("Continuing Conversation");
        console.info(&format!("Message: {user_input}"));

        if let Some(task) = conversation.task.clone() {
            // Continue with existing task context
            execute_conversation_continuation(console, sdk, conversation, &task).await
        } else {
            // Fallback: create new task if somehow task is missing
            let working_dir = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string();

            let task = TaskMetadata::new(user_input, &working_dir);
            conversation.task = Some(task.clone());
            execute_conversation_task(console, sdk, conversation, &task).await
        }
    }
}

/// Execute a new conversation task
async fn execute_conversation_task(
    console: &CLIConsole,
    sdk: &SageAgentSDK,
    conversation: &mut ConversationSession,
    task: &TaskMetadata,
) -> SageResult<()> {
    let start_time = std::time::Instant::now();

    console.info("🤔 Starting conversation...");

    // Set state to executing task
    set_global_app_state(AppState::ExecutingTask);

    let run_options = RunOptions::new().with_trajectory(true);

    match tokio::time::timeout(
        std::time::Duration::from_secs(300),
        sdk.run_with_options(&task.description, run_options),
    )
    .await
    {
        Ok(result) => {
            match result {
                Ok(execution_result) => {
                    let duration = start_time.elapsed();
                    conversation.execution = Some(execution_result.execution().clone());
                    conversation.mark_first_message_processed();

                    // Add assistant response to conversation
                    if let Some(final_result) = &execution_result.execution().final_result {
                        conversation.add_assistant_message(final_result);
                    }

                    // Handle different execution outcomes
                    match &execution_result.outcome {
                        ExecutionOutcome::Success(_) => {
                            console.success("✓ Task completed successfully!");
                        }
                        ExecutionOutcome::Failed { error, .. } => {
                            console.error("✗ Task failed!");
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
                                _ => {}
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
                            console.warn("🛑 Task interrupted by user");
                            console
                                .info("ℹ You can continue with a new task or type 'exit' to quit");
                        }
                        ExecutionOutcome::MaxStepsReached { .. } => {
                            console.warn("⚠ Task reached maximum steps without completion");
                            console
                                .info("ℹ Consider breaking down the task or increasing max_steps");
                        }
                    }

                    console.info(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
                    console.info(&format!(
                        "ℹ Steps: {}",
                        execution_result.execution().steps.len()
                    ));
                    console.info(&format!(
                        "ℹ Tokens: {}",
                        execution_result.execution().total_usage.total_tokens
                    ));

                    if let Some(trajectory_path) = &execution_result.trajectory_path {
                        console.info(&format!(
                            "ℹ Trajectory saved: {}",
                            trajectory_path.display()
                        ));
                    }

                    Ok(())
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    console.error("✗ System error!");
                    console.error(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
                    console.error(&format!("ℹ Error: {e}"));
                    Err(e)
                }
            }
        }
        Err(_) => {
            let duration = start_time.elapsed();
            console.error(&format!(
                "Conversation timed out after {:.2}s",
                duration.as_secs_f64()
            ));
            Err(SageError::Timeout { seconds: 300 })
        }
    }
}

/// Execute conversation continuation (for follow-up messages)
async fn execute_conversation_continuation(
    console: &CLIConsole,
    sdk: &SageAgentSDK,
    conversation: &mut ConversationSession,
    _task: &TaskMetadata,
) -> SageResult<()> {
    let start_time = std::time::Instant::now();

    console.info("🤔 Continuing conversation...");

    // Get the last user message
    let user_message = conversation
        .messages
        .last()
        .map(|msg| msg.content.as_str())
        .unwrap_or("No message");

    // Set state to executing task
    set_global_app_state(AppState::ExecutingTask);

    // Get the current execution, if it exists
    if let Some(execution) = &mut conversation.execution {
        match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            sdk.continue_execution(execution, user_message),
        )
        .await
        {
            Ok(result) => {
                match result {
                    Ok(()) => {
                        let duration = start_time.elapsed();

                        // Get execution info before borrowing conversation again
                        let final_result = execution.final_result.clone();
                        let steps_len = execution.steps.len();
                        let total_tokens = execution.total_usage.total_tokens;

                        // Add assistant response to conversation
                        if let Some(final_result) = final_result {
                            conversation.add_assistant_message(&final_result);
                        }

                        console.success("✓ Conversation continued successfully!");
                        console.info(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
                        console.info(&format!("ℹ Steps: {}", steps_len));
                        console.info(&format!("ℹ Tokens: {}", total_tokens));

                        Ok(())
                    }
                    Err(e) => {
                        let duration = start_time.elapsed();

                        // Check if this was an interruption
                        if e.to_string().contains("interrupted") {
                            console.warn("🛑 Task interrupted by user");
                            console
                                .info(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
                            console
                                .info("ℹ You can continue with a new task or type 'exit' to quit");
                            Ok(()) // Don't treat interruption as an error in interactive mode
                        } else {
                            console.error("✗ Conversation continuation failed!");
                            console.error(&format!(
                                "ℹ Execution time: {:.2}s",
                                duration.as_secs_f64()
                            ));
                            console.error(&format!("ℹ Error: {e}"));
                            Err(e)
                        }
                    }
                }
            }
            Err(_) => {
                let duration = start_time.elapsed();
                console.error(&format!(
                    "Conversation continuation timed out after {:.2}s",
                    duration.as_secs_f64()
                ));
                Err(SageError::Timeout { seconds: 300 })
            }
        }
    } else {
        console.error("No existing execution to continue");
        Err(SageError::InvalidInput(
            "No existing execution to continue".to_string(),
        ))
    }
}
