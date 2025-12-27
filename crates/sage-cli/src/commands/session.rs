//! Session management CLI commands
//!
//! Commands for listing, resuming, and deleting conversation sessions.

use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};
use sage_core::error::SageResult;
use sage_core::session::JsonlSessionStorage;

use super::session_resume::{print_session_details, SessionSelector};
use crate::args::SessionAction;

/// Execute session command
pub async fn execute(action: SessionAction) -> SageResult<()> {
    match action {
        SessionAction::List { all, branch, limit } => list_sessions(all, branch, limit).await,
        SessionAction::Resume { session_id, all } => resume_session(session_id, all).await,
        SessionAction::Delete { session_id, force } => delete_session(&session_id, force).await,
        SessionAction::Show { session_id } => show_session(&session_id).await,
    }
}

/// List available sessions
async fn list_sessions(all: bool, branch: Option<String>, limit: usize) -> SageResult<()> {
    let selector = SessionSelector::new()?.show_all_projects(all);
    let mut sessions = selector.list_sessions().await?;

    // Filter by branch if specified
    if let Some(ref branch_filter) = branch {
        sessions.retain(|s| s.git_branch.as_ref() == Some(branch_filter));
    }

    // Filter out sidechain sessions by default
    sessions.retain(|s| !s.is_sidechain);

    // Apply limit
    sessions.truncate(limit);

    if sessions.is_empty() {
        println!("{}", "No sessions found.".yellow());
        if !all {
            println!(
                "{}",
                "Tip: Use --all to see sessions from all projects.".dimmed()
            );
        }
        if branch.is_some() {
            println!(
                "{}",
                "Tip: Remove --branch filter to see all sessions.".dimmed()
            );
        }
        return Ok(());
    }

    println!("\n{}", "Sessions".bold().underline());
    println!(
        "{}",
        format!("Showing {} session(s)", sessions.len()).dimmed()
    );
    println!();

    for session in &sessions {
        // Use display_title() for Claude Code-style title display
        let display_name = session.display_title();
        let display_name = if display_name.len() > 50 {
            format!("{}...", &display_name[..47])
        } else {
            display_name.to_string()
        };

        // Get relative time
        let time_str = format_relative_time(session.updated_at);

        // Format branch info
        let branch_str = session
            .git_branch
            .as_ref()
            .map(|b| format!(" [{}]", b.green()))
            .unwrap_or_default();

        // Format directory
        let dir_name = session
            .working_directory
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| session.working_directory.to_string_lossy().to_string());

        // Show session ID prefix and title
        println!(
            "  {} {}{}",
            session.id[..8].bright_cyan(),
            display_name.bright_white(),
            branch_str
        );

        // Show summary if available (Claude Code style)
        if let Some(ref summary) = session.summary {
            let summary_preview = if summary.len() > 60 {
                format!("{}...", &summary[..57])
            } else {
                summary.clone()
            };
            println!("    {}", summary_preview.italic().dimmed());
        }

        println!(
            "    {} {} {} {}",
            format!("{} msgs", session.message_count).dimmed(),
            format!("in {}", dir_name).dimmed(),
            "|".dimmed(),
            time_str.dimmed()
        );
    }

    println!();
    println!(
        "{}",
        "Use 'sage session resume <id>' to resume a session.".dimmed()
    );

    Ok(())
}

/// Resume a session
async fn resume_session(session_id: Option<String>, all: bool) -> SageResult<()> {
    let selector = SessionSelector::new()?.show_all_projects(all);

    let result = if let Some(id) = session_id {
        selector.resume_by_id(&id).await?
    } else {
        selector.select_session().await?
    };

    match result {
        Some(resume_result) => {
            print_session_details(&resume_result.metadata);

            // Handle cross-project detection (Claude Code style)
            if resume_result.is_cross_project {
                println!(
                    "\n{}",
                    "⚠ Cross-project session detected!".yellow().bold()
                );
                println!(
                    "{}",
                    format!(
                        "This session was created in: {}",
                        resume_result.working_directory.display()
                    )
                    .dimmed()
                );
                println!(
                    "{}",
                    format!(
                        "Current directory: {}",
                        std::env::current_dir()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| "unknown".to_string())
                    )
                    .dimmed()
                );

                if let Some(ref cmd) = resume_result.cross_project_command {
                    println!("\n{}", "To resume in the correct directory, run:".dimmed());
                    println!("  {}", cmd.bright_cyan());
                }
            } else {
                println!(
                    "{}",
                    format!(
                        "Session {} ready to resume.",
                        resume_result.session_id[..8].bright_cyan()
                    )
                );
                println!(
                    "{}",
                    "Run 'sage interactive' to continue this session.".dimmed()
                );
            }

            Ok(())
        }
        None => Ok(()),
    }
}

/// Delete a session
async fn delete_session(session_id: &str, force: bool) -> SageResult<()> {
    let storage = JsonlSessionStorage::default_path()?;

    // Check if session exists
    let metadata = storage.load_metadata(&session_id.to_string()).await?;
    if metadata.is_none() {
        println!(
            "{}",
            format!("Session '{}' not found.", session_id).red()
        );
        return Ok(());
    }

    let metadata = metadata.unwrap();

    // Confirm deletion unless --force is used
    if !force {
        print_session_details(&metadata);

        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Are you sure you want to delete session '{}'?",
                session_id
            ))
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirm {
            println!("{}", "Deletion cancelled.".dimmed());
            return Ok(());
        }
    }

    // Delete the session
    storage.delete_session(&session_id.to_string()).await?;
    println!(
        "{}",
        format!("Session '{}' deleted successfully.", session_id).green()
    );

    Ok(())
}

/// Show session details
async fn show_session(session_id: &str) -> SageResult<()> {
    let storage = JsonlSessionStorage::default_path()?;

    match storage.load_metadata(&session_id.to_string()).await? {
        Some(metadata) => {
            print_session_details(&metadata);

            // Also show first few messages
            let messages = storage.load_messages(&session_id.to_string()).await?;
            if !messages.is_empty() {
                println!("{}", "Recent Messages:".bold());
                for (i, msg) in messages.iter().take(5).enumerate() {
                    let role = &msg.message.role;
                    let content = &msg.message.content;
                    let role_str = match role.as_str() {
                        "user" => "User".green(),
                        "assistant" => "Assistant".cyan(),
                        _ => role.clone().dimmed(),
                    };
                    let preview = if content.len() > 80 {
                        format!("{}...", &content[..77])
                    } else {
                        content.clone()
                    };
                    println!("  {}. {}: {}", i + 1, role_str, preview.dimmed());
                }
                if messages.len() > 5 {
                    println!(
                        "  {}",
                        format!("... and {} more messages", messages.len() - 5).dimmed()
                    );
                }
            }

            Ok(())
        }
        None => {
            println!(
                "{}",
                format!("Session '{}' not found.", session_id).red()
            );
            Ok(())
        }
    }
}

/// Format relative time like "5 minutes ago", "2 hours ago", etc.
fn format_relative_time(time: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(time);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if duration.num_days() < 7 {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if duration.num_weeks() < 4 {
        let weeks = duration.num_weeks();
        format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
    } else {
        let months = duration.num_days() / 30;
        if months < 12 {
            format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
        } else {
            let years = months / 12;
            format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
        }
    }
}
