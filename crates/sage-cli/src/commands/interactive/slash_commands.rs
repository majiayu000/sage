//! Slash command handling

use super::conversation::handle_conversation;
use super::resume::handle_resume_command;
use super::session::ConversationSession;
use crate::console::CliConsole;
use crate::signal_handler::{AppState, set_global_app_state};
use sage_core::commands::types::InteractiveCommand;
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::error::SageResult;
use sage_sdk::SageAgentSdk;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Handle slash commands in interactive mode
/// Returns Ok(true) if the command was handled, Ok(false) if it should be treated as conversation
pub async fn handle_slash_command(
    console: &CliConsole,
    sdk: &SageAgentSdk,
    conversation: &mut ConversationSession,
    input: &str,
) -> SageResult<bool> {
    let working_dir = std::env::current_dir().unwrap_or_default();

    let mut registry = CommandRegistry::new(&working_dir);
    registry.register_builtins();
    if let Err(e) = registry.discover().await {
        console.warn(&format!("Failed to discover commands: {}", e));
    }

    let cmd_executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));

    match cmd_executor.process(input).await {
        Ok(Some(result)) => {
            if let Some(interactive_cmd) = &result.interactive {
                handle_interactive_command(interactive_cmd, console, conversation).await?;
                return Ok(true);
            }

            if result.is_local {
                if let Some(status) = &result.status_message {
                    console.info(status);
                }
                if let Some(output) = &result.local_output {
                    println!("{}", output);
                }
                return Ok(true);
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

            if result.expanded_prompt.contains("__CLEAR_CONVERSATION__") {
                conversation.reset();
                console.success("Conversation cleared!");
                return Ok(true);
            }

            set_global_app_state(AppState::ExecutingTask);
            handle_conversation(console, sdk, conversation, &result.expanded_prompt).await?;
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Handle interactive commands that require CLI interaction
async fn handle_interactive_command(
    cmd: &InteractiveCommand,
    console: &CliConsole,
    conversation: &mut ConversationSession,
) -> SageResult<()> {
    match cmd {
        InteractiveCommand::Resume {
            session_id,
            show_all,
        } => handle_resume_command(session_id.clone(), *show_all, console).await,
        InteractiveCommand::Title { title } => {
            handle_title_command(title, console, conversation).await
        }
    }
}

/// Handle /title command - set custom session title
async fn handle_title_command(
    title: &str,
    console: &CliConsole,
    conversation: &ConversationSession,
) -> SageResult<()> {
    use sage_core::session::JsonlSessionStorage;

    // Get current session ID
    let session_id = match conversation.session_id() {
        Some(id) => id.to_string(),
        None => {
            console.warn("No active session. Start a conversation first.");
            return Ok(());
        }
    };

    // Update session metadata with custom title
    let storage = JsonlSessionStorage::default_path()?;
    if let Ok(Some(mut metadata)) = storage.load_metadata(&session_id).await {
        metadata.set_custom_title(title);
        if storage.save_metadata(&session_id, &metadata).await.is_ok() {
            console.success(&format!("Session title set to: {}", title));
        } else {
            console.warn("Failed to save session title.");
        }
    } else {
        console.warn("Could not load session metadata.");
    }

    Ok(())
}
