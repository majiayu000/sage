//! Slash command handling

use super::conversation::handle_conversation;
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
        } => {
            handle_resume_command(session_id.as_deref(), *show_all, console).await
        }
        InteractiveCommand::Title { title } => {
            handle_title_command(title, console, conversation).await
        }
    }
}

/// Handle /resume command - show and select sessions to resume
async fn handle_resume_command(
    session_id: Option<&str>,
    show_all: bool,
    console: &CliConsole,
) -> SageResult<()> {
    use sage_core::session::JsonlSessionStorage;

    let storage = JsonlSessionStorage::default_path()?;
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
            println!("  Last msg: {}", session.resume_title());
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
        // Use resume_title() to show last user input
        let title = session.resume_title();
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
