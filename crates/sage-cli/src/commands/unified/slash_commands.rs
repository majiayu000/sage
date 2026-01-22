//! Slash command processing for the unified command

use crate::console::CliConsole;
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::error::SageResult;
use sage_core::output::OutputMode;
use std::sync::Arc;

/// Result of processing a slash command
pub enum SlashCommandAction {
    /// Send this prompt to the LLM
    Prompt(String),
    /// Command was handled locally, no further action needed
    Handled,
    /// Set output mode
    SetOutputMode(OutputMode),
    /// Resume a session
    Resume { session_id: Option<String> },
}

/// Process slash commands
pub async fn process_slash_command(
    input: &str,
    console: &CliConsole,
    working_dir: &std::path::Path,
) -> SageResult<SlashCommandAction> {
    if !CommandExecutor::is_command(input) {
        return Ok(SlashCommandAction::Prompt(input.to_string()));
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
                return handle_interactive_command_v2(&interactive_cmd, console).await;
            }

            // Handle local commands (output directly, no LLM)
            if result.is_local {
                if let Some(status) = &result.status_message {
                    console.info(status);
                }
                if let Some(output) = &result.local_output {
                    println!("{}", output);
                }
                return Ok(SlashCommandAction::Handled);
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
            Ok(SlashCommandAction::Prompt(result.expanded_prompt))
        }
        Ok(None) => Ok(SlashCommandAction::Prompt(input.to_string())),
        Err(e) => Err(e),
    }
}

/// Handle interactive commands, returning the appropriate action
pub async fn handle_interactive_command_v2(
    cmd: &sage_core::commands::types::InteractiveCommand,
    console: &CliConsole,
) -> SageResult<SlashCommandAction> {
    use sage_core::commands::types::InteractiveCommand;

    match cmd {
        InteractiveCommand::Resume { session_id, .. } => {
            Ok(SlashCommandAction::Resume { session_id: session_id.clone() })
        }
        InteractiveCommand::Title { title } => {
            console.warn(&format!(
                "Title command not available in non-interactive mode. Title: {}",
                title
            ));
            Ok(SlashCommandAction::Handled)
        }
        InteractiveCommand::Login => {
            // Run the login flow directly
            use crate::commands::interactive::CliOnboarding;

            let mut onboarding = CliOnboarding::new();
            match onboarding.run_login().await {
                Ok(true) => {
                    console.success("API key updated! Restart sage to use the new key.");
                }
                Ok(false) => {
                    console.info("API key not changed.");
                }
                Err(e) => {
                    console.error(&format!("Login failed: {}", e));
                }
            }
            Ok(SlashCommandAction::Handled)
        }
        InteractiveCommand::OutputMode { mode } => {
            let output_mode = match mode.as_str() {
                "streaming" => OutputMode::Streaming,
                "batch" => OutputMode::Batch,
                "silent" => OutputMode::Silent,
                _ => {
                    console.warn(&format!("Unknown output mode: {}", mode));
                    return Ok(SlashCommandAction::Handled);
                }
            };
            Ok(SlashCommandAction::SetOutputMode(output_mode))
        }
    }
}
