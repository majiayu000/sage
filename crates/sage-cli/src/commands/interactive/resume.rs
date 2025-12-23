//! Session resume command handling

use crate::commands::session_resume::{SessionSelector, print_session_details};
use crate::console::CliConsole;
use sage_core::error::{SageError, SageResult};

/// Handle the /resume command with interactive session selection
pub async fn handle_resume_command(
    session_id: Option<String>,
    show_all: bool,
    console: &CliConsole,
) -> SageResult<()> {
    let selector = SessionSelector::new()?.show_all_projects(show_all);

    if let Some(id) = session_id {
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

                    let messages = selector.storage().load_messages(&result.session_id).await?;
                    if !messages.is_empty() {
                        console.info(&format!(
                            "Session has {} messages. Ready to continue.",
                            messages.len()
                        ));

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

                    console.info("\nSession loaded. You can continue the conversation now.");
                    return Ok(());
                }
                None => {
                    return Err(SageError::not_found(format!("Session '{}' not found", id)));
                }
            }
        }
    }

    match selector.select_session().await? {
        Some(result) => {
            console.success(&format!("Selected session: {}", result.session_id));
            print_session_details(&result.metadata);

            let current_dir = std::env::current_dir().unwrap_or_default();
            if result.working_directory != current_dir {
                console.warn("This session is from a different directory.");
                console.info(&format!(
                    "Session directory: {}",
                    result.working_directory.display()
                ));
                console.info(&format!("Current directory: {}", current_dir.display()));
                console.info("\nYou may need to restart sage in that directory to fully resume.");
            } else {
                console.info("\nSession loaded. You can continue the conversation now.");
            }

            Ok(())
        }
        None => Ok(()),
    }
}
