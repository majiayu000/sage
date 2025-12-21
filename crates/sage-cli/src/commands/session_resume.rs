//! Interactive session resume functionality
//!
//! This module provides interactive session selection and resume functionality,
//! similar to Claude Code's /resume command.

use chrono::{DateTime, Utc};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use sage_core::error::{SageError, SageResult};
use sage_core::session::{JsonlSessionStorage, SessionMetadata};
use std::path::PathBuf;

/// Session resume result
#[derive(Debug, Clone)]
pub struct ResumeResult {
    /// Selected session ID
    pub session_id: String,
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Working directory of the session
    pub working_directory: PathBuf,
}

/// Interactive session selector
pub struct SessionSelector {
    storage: JsonlSessionStorage,
    show_all_projects: bool,
}

impl SessionSelector {
    /// Create a new session selector with default storage path
    pub fn new() -> SageResult<Self> {
        let storage = JsonlSessionStorage::default_path()?;
        Ok(Self {
            storage,
            show_all_projects: false,
        })
    }

    /// Create a new session selector with custom storage path
    pub fn with_storage_path(path: impl Into<PathBuf>) -> Self {
        Self {
            storage: JsonlSessionStorage::new(path),
            show_all_projects: false,
        }
    }

    /// Set whether to show sessions from all projects
    pub fn show_all_projects(mut self, show_all: bool) -> Self {
        self.show_all_projects = show_all;
        self
    }

    /// List all available sessions
    pub async fn list_sessions(&self) -> SageResult<Vec<SessionMetadata>> {
        let mut sessions = self.storage.list_sessions().await?;

        // Filter by current working directory if not showing all projects
        if !self.show_all_projects {
            if let Ok(current_dir) = std::env::current_dir() {
                sessions.retain(|s| s.working_directory == current_dir);
            }
        }

        Ok(sessions)
    }

    /// Interactively select a session to resume
    pub async fn select_session(&self) -> SageResult<Option<ResumeResult>> {
        let sessions = self.list_sessions().await?;

        if sessions.is_empty() {
            println!("{}", "No sessions found to resume.".yellow());
            if !self.show_all_projects {
                println!(
                    "{}",
                    "Tip: Use /resume --all to see sessions from all projects.".dimmed()
                );
            }
            return Ok(None);
        }

        // Format session items for display
        let items: Vec<String> = sessions
            .iter()
            .map(|s| format_session_item(s))
            .collect();

        // Add cancel option
        let mut display_items = items.clone();
        display_items.push("Cancel".dimmed().to_string());

        println!("\n{}", "Select a session to resume:".bold());
        println!("{}", "Use arrow keys to navigate, Enter to select, or type to search".dimmed());
        println!();

        // Use FuzzySelect for better UX
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(&display_items)
            .default(0)
            .highlight_matches(true)
            .interact_opt()
            .map_err(|e| SageError::Io(format!("Failed to display session selector: {}", e)))?;

        match selection {
            Some(idx) if idx < sessions.len() => {
                let session = &sessions[idx];
                Ok(Some(ResumeResult {
                    session_id: session.id.clone(),
                    metadata: session.clone(),
                    working_directory: session.working_directory.clone(),
                }))
            }
            _ => {
                println!("{}", "Resume cancelled.".dimmed());
                Ok(None)
            }
        }
    }

    /// Resume a session by ID directly
    pub async fn resume_by_id(&self, session_id: &str) -> SageResult<Option<ResumeResult>> {
        let id = session_id.to_string();
        match self.storage.load_metadata(&id).await? {
            Some(metadata) => Ok(Some(ResumeResult {
                session_id: session_id.to_string(),
                metadata: metadata.clone(),
                working_directory: metadata.working_directory,
            })),
            None => {
                println!(
                    "{}",
                    format!("Session '{}' not found.", session_id).red()
                );
                Ok(None)
            }
        }
    }

    /// Get the underlying storage for loading messages
    pub fn storage(&self) -> &JsonlSessionStorage {
        &self.storage
    }
}

/// Format a session item for display
fn format_session_item(session: &SessionMetadata) -> String {
    let time_str = format_relative_time(session.updated_at);

    // Truncate session name or ID for display
    let name = session.name.as_deref().unwrap_or(&session.id);
    let display_name = if name.len() > 40 {
        format!("{}...", &name[..37])
    } else {
        name.to_string()
    };

    // Get working directory basename
    let dir_name = session
        .working_directory
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| session.working_directory.to_string_lossy().to_string());

    // Format with model and message count
    let model_str = session.model.as_deref().unwrap_or("unknown");

    format!(
        "{} {} {} {} {}",
        display_name.bright_white(),
        format!("[{}]", dir_name).dimmed(),
        format!("({})", model_str).cyan(),
        format!("{} msgs", session.message_count).dimmed(),
        time_str.dimmed()
    )
}

/// Format relative time like "5 minutes ago", "2 hours ago", etc.
fn format_relative_time(time: DateTime<Utc>) -> String {
    let now = Utc::now();
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

/// Print session details
pub fn print_session_details(session: &SessionMetadata) {
    println!("\n{}", "Session Details".bold().underline());
    println!("  {} {}", "ID:".dimmed(), session.id.bright_white());
    if let Some(name) = &session.name {
        println!("  {} {}", "Name:".dimmed(), name.bright_white());
    }
    println!(
        "  {} {}",
        "Directory:".dimmed(),
        session.working_directory.display().to_string().cyan()
    );
    if let Some(branch) = &session.git_branch {
        println!("  {} {}", "Git Branch:".dimmed(), branch.green());
    }
    if let Some(model) = &session.model {
        println!("  {} {}", "Model:".dimmed(), model.cyan());
    }
    println!(
        "  {} {}",
        "Messages:".dimmed(),
        session.message_count.to_string().yellow()
    );
    println!(
        "  {} {}",
        "State:".dimmed(),
        format_state(&session.state)
    );
    println!(
        "  {} {}",
        "Created:".dimmed(),
        format_relative_time(session.created_at)
    );
    println!(
        "  {} {}",
        "Updated:".dimmed(),
        format_relative_time(session.updated_at)
    );
    println!();
}

/// Format session state with color
fn format_state(state: &str) -> String {
    match state {
        "active" => state.green().to_string(),
        "completed" => state.blue().to_string(),
        "failed" => state.red().to_string(),
        _ => state.dimmed().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_selector_empty() {
        let tmp = TempDir::new().unwrap();
        let selector = SessionSelector::with_storage_path(tmp.path());

        let sessions = selector.list_sessions().await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_format_relative_time() {
        let now = Utc::now();
        assert_eq!(format_relative_time(now), "just now");

        let one_hour_ago = now - chrono::Duration::hours(1);
        assert_eq!(format_relative_time(one_hour_ago), "1 hour ago");

        let two_days_ago = now - chrono::Duration::days(2);
        assert_eq!(format_relative_time(two_days_ago), "2 days ago");
    }
}
