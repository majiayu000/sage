//! Session resume command handling

use crate::commands::session_resume::{print_session_details, SessionSelector};
use crate::console::CliConsole;
use sage_core::commands::types::InteractiveCommand;
use sage_core::error::{SageError, SageResult};
use sage_core::session::EnhancedMessage;

/// Handle interactive commands that require CLI interaction
pub async fn handle_interactive_command(
    cmd: &InteractiveCommand,
    console: &CliConsole,
) -> SageResult<()> {
    match cmd {
        InteractiveCommand::Resume {
            session_id,
            show_all,
        } => handle_resume_command(session_id.clone(), *show_all, console).await,
        InteractiveCommand::Title { .. } => {
            console.warn("The /title command is only available in interactive mode.");
            Ok(())
        }
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
            return resume_by_id(&selector, &id, console).await;
        }
    }

    // Interactive session selection
    interactive_session_selection(&selector, console).await
}

/// Resume a session by its ID
async fn resume_by_id(
    selector: &SessionSelector,
    id: &str,
    console: &CliConsole,
) -> SageResult<()> {
    match selector.resume_by_id(id).await? {
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
                display_recent_messages(&messages, console);
            }

            // Session resumption info
            console.info("\nTo continue this session, run:");
            console.info(&format!(
                "  sage run --session {} \"<your message>\"",
                result.session_id
            ));

            Ok(())
        }
        None => Err(SageError::not_found(format!("Session '{}' not found", id))),
    }
}

/// Display recent messages from a session
fn display_recent_messages(messages: &[EnhancedMessage], console: &CliConsole) {
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

/// Interactive session selection flow
async fn interactive_session_selection(
    selector: &SessionSelector,
    console: &CliConsole,
) -> SageResult<()> {
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
