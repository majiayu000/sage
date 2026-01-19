//! Slash command processing for the unified command

use crate::console::CliConsole;
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::error::SageResult;
use sage_core::output::OutputMode;
use sage_core::session::JsonlSessionStorage;
use std::sync::Arc;

use super::utils::{format_time_ago, truncate_str};

/// Result of processing a slash command
pub enum SlashCommandAction {
    /// Send this prompt to the LLM
    Prompt(String),
    /// Command was handled locally, no further action needed
    Handled,
    /// Resume a session with the given ID
    ResumeSession(String),
    /// Set output mode
    SetOutputMode(OutputMode),
}

/// Process slash commands
pub async fn process_slash_command(
    input: &str,
    console: &CliConsole,
    working_dir: &std::path::Path,
    jsonl_storage: &Arc<JsonlSessionStorage>,
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
                return handle_interactive_command_v2(&interactive_cmd, console, jsonl_storage)
                    .await;
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
    storage: &Arc<JsonlSessionStorage>,
) -> SageResult<SlashCommandAction> {
    use sage_core::commands::types::InteractiveCommand;

    match cmd {
        InteractiveCommand::Resume {
            session_id,
            show_all,
        } => handle_resume_interactive(session_id.as_deref(), *show_all, console, storage).await,
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

/// Handle /resume command with interactive selection
pub async fn handle_resume_interactive(
    session_id: Option<&str>,
    show_all: bool,
    console: &CliConsole,
    storage: &Arc<JsonlSessionStorage>,
) -> SageResult<SlashCommandAction> {
    use dialoguer::{theme::ColorfulTheme, Select};

    let sessions = storage.list_sessions().await?;

    if sessions.is_empty() {
        console.info("No previous sessions found.");
        console.info("Start a conversation to create a new session.");
        return Ok(SlashCommandAction::Handled);
    }

    // If a specific session ID was provided, resume it directly
    if let Some(id) = session_id {
        if let Some(session) = sessions
            .iter()
            .find(|s| s.id == id || s.id.starts_with(id))
        {
            console.success(&format!("Resuming session: {}", session.resume_title()));
            return Ok(SlashCommandAction::ResumeSession(session.id.clone()));
        } else {
            console.warn(&format!("Session not found: {}", id));
            return Ok(SlashCommandAction::Handled);
        }
    }

    // Build selection items
    let display_count = if show_all {
        sessions.len()
    } else {
        10.min(sessions.len())
    };

    let items: Vec<String> = sessions
        .iter()
        .take(display_count)
        .map(|s| {
            let title = truncate_str(s.resume_title(), 50);
            let time_ago = format_time_ago(&s.updated_at);
            format!("{} ({}, {} msgs)", title, time_ago, s.message_count)
        })
        .collect();

    // Add cancel option
    let mut items_with_cancel = items.clone();
    items_with_cancel.push("Cancel".to_string());

    println!();
    console.info("Select a session to resume (↑/↓ to navigate, Enter to select):");
    println!();

    // Interactive selection
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items_with_cancel)
        .default(0)
        .interact_opt();

    match selection {
        Ok(Some(idx)) if idx < sessions.len().min(display_count) => {
            let session = &sessions[idx];
            console.success(&format!("Resuming: {}", session.resume_title()));
            Ok(SlashCommandAction::ResumeSession(session.id.clone()))
        }
        _ => {
            console.info("Cancelled.");
            Ok(SlashCommandAction::Handled)
        }
    }
}
