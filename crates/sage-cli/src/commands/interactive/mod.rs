//! Interactive mode implementation

mod conversation;
mod execution;
mod help;
mod outcome;
mod resume;
mod session;
mod slash_commands;

use crate::console::CliConsole;
use crate::signal_handler::{AppState, set_global_app_state, start_global_signal_handling};
use conversation::handle_conversation;
use help::{print_config, print_help, print_input_help, print_status};
use sage_core::config::{format_api_key_status_for_provider, ApiKeySource};
use sage_core::error::{SageError, SageResult};
use sage_core::ui::EnhancedConsole;
use sage_sdk::SageAgentSdk;
use session::ConversationSession;
use slash_commands::handle_slash_command;
use std::io::Write;
use std::path::PathBuf;

/// Arguments for interactive mode
pub struct InteractiveArgs {
    pub config_file: String,
    pub trajectory_file: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
}

/// Execute interactive mode
pub async fn execute(args: InteractiveArgs) -> SageResult<()> {
    let console = CliConsole::new(true);

    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    EnhancedConsole::print_welcome_banner();
    EnhancedConsole::print_section_header(
        "Interactive Mode",
        Some("Type 'help' for available commands, 'exit' to quit"),
    );

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

    if let Some(working_dir) = &args.working_dir {
        sdk = sdk.with_working_directory(working_dir);
    }

    if let Some(trajectory_file) = &args.trajectory_file {
        sdk = sdk.with_trajectory_path(trajectory_file);
    }

    // Display API key status for default provider
    let config = sdk.config();
    let default_provider = config.get_default_provider();
    if let Some(params) = config.model_providers.get(default_provider) {
        let key_info = params.get_api_key_info_for_provider(&default_provider);
        let status_msg = format_api_key_status_for_provider(&default_provider, &key_info);

        match key_info.source {
            ApiKeySource::NotFound => {
                console.warn(&status_msg);
                console.info(&format!(
                    "Hint: Set {} environment variable or add to config file",
                    format!("{}_API_KEY", default_provider.to_uppercase())
                ));
            }
            _ => {
                console.success(&status_msg);
                // Validate key format
                if let Err(e) = params.validate_api_key_format_for_provider(&default_provider) {
                    console.warn(&format!("API key warning: {}", e));
                }
            }
        }
    }

    console.success("Interactive mode initialized");
    console.print_separator();

    let mut conversation = ConversationSession::new();

    loop {
        std::io::stdout().flush().unwrap_or(());
        std::io::stderr().flush().unwrap_or(());

        set_global_app_state(AppState::WaitingForInput);

        match console.input("sage") {
            Ok(input) => {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

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
                        print!("\x1B[2J\x1B[1;1H");
                        print!("\x1B[3J");
                        console.success("Screen cleared!");
                    }
                    "reset" | "refresh" => {
                        print!("\r\x1B[K");
                        print!("\x1B[2J\x1B[1;1H");
                        print!("\x1B[3J");
                        console.success("Terminal display reset!");
                    }
                    "input-help" | "ih" => {
                        print_input_help(&console);
                    }
                    "new" | "new-task" => {
                        conversation.reset();
                        console.success("Started new conversation. Previous context cleared.");
                    }
                    "conversation" | "conv" => {
                        console.info(&format!(
                            "Current conversation: {}",
                            conversation.get_summary()
                        ));
                    }
                    _ => {
                        if input.starts_with('/') {
                            match handle_slash_command(&console, &sdk, &mut conversation, input)
                                .await
                            {
                                Ok(true) => {
                                    continue;
                                }
                                Ok(false) => {}
                                Err(e) => {
                                    console.error(&format!("Command error: {e}"));
                                    continue;
                                }
                            }
                        }

                        set_global_app_state(AppState::ExecutingTask);

                        match handle_conversation(&console, &sdk, &mut conversation, input).await {
                            Ok(()) => {}
                            Err(e) => {
                                console.error(&format!("Conversation failed: {e}"));

                                if is_critical_error(&e) {
                                    console.error(
                                        "Critical error encountered. Exiting interactive mode.",
                                    );
                                    break;
                                }

                                console.info(
                                    "You can try again or type 'help' for available commands.",
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::Interrupted
                ) {
                    break;
                }
                console.error(&format!("Input error: {e}"));
                console.warn("Input error occurred. Please try again.");
                continue;
            }
        }

        console.print_separator();
    }

    Ok(())
}

/// Check if an error is critical and should terminate the interactive session
fn is_critical_error(error: &SageError) -> bool {
    match error {
        SageError::Config { .. } => true,
        SageError::Llm { .. } => false,
        SageError::Tool { .. } => false,
        SageError::Agent { .. } => false,
        SageError::Io { .. } => false,
        SageError::Json { .. } => false,
        SageError::Http { .. } => false,
        SageError::InvalidInput { .. } => false,
        SageError::Timeout { .. } => false,
        SageError::Cancelled => false,
        _ => false,
    }
}
