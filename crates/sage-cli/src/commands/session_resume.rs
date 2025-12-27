//! Interactive session resume functionality
//!
//! This module provides interactive session selection and resume functionality,
//! similar to Claude Code's /resume command.

use chrono::{DateTime, Utc};
use colored::Colorize;
use dialoguer::{FuzzySelect, theme::ColorfulTheme};
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
    /// Whether this is a cross-project resume
    pub is_cross_project: bool,
    /// Command to run for cross-project resume
    pub cross_project_command: Option<String>,
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
    #[allow(dead_code)] // Used in tests and for custom storage paths
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
        let items: Vec<String> = sessions.iter().map(|s| format_session_item(s)).collect();

        // Add cancel option
        let mut display_items = items.clone();
        display_items.push("Cancel".dimmed().to_string());

        println!("\n{}", "Select a session to resume:".bold());
        println!(
            "{}",
            "Use arrow keys to navigate, Enter to select, or type to search".dimmed()
        );
        println!();

        // Use FuzzySelect for better UX
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(&display_items)
            .default(0)
            .highlight_matches(true)
            .interact_opt()
            .map_err(|e| SageError::io(format!("Failed to display session selector: {}", e)))?;

        match selection {
            Some(idx) if idx < sessions.len() => {
                let session = &sessions[idx];
                Ok(Some(self.create_resume_result(session)))
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
            Some(metadata) => Ok(Some(self.create_resume_result(&metadata))),
            None => {
                println!("{}", format!("Session '{}' not found.", session_id).red());
                Ok(None)
            }
        }
    }

    /// Create a ResumeResult with cross-project detection
    fn create_resume_result(&self, metadata: &SessionMetadata) -> ResumeResult {
        let current_dir = std::env::current_dir().ok();
        let session_dir = &metadata.working_directory;

        let is_cross_project = current_dir
            .as_ref()
            .map(|cd| cd != session_dir)
            .unwrap_or(false);

        let cross_project_command = if is_cross_project {
            Some(format!(
                "cd {} && sage session resume {}",
                session_dir.display(),
                &metadata.id[..8]
            ))
        } else {
            None
        };

        ResumeResult {
            session_id: metadata.id.clone(),
            metadata: metadata.clone(),
            working_directory: metadata.working_directory.clone(),
            is_cross_project,
            cross_project_command,
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

    // Use display_title() for Claude Code-style title display
    let display_name = session.display_title();
    let display_name = if display_name.len() > 40 {
        format!("{}...", &display_name[..37])
    } else {
        display_name.to_string()
    };

    // Get working directory basename
    let dir_name = session
        .working_directory
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| session.working_directory.to_string_lossy().to_string());

    // Format with model and message count
    let model_str = session.model.as_deref().unwrap_or("unknown");

    // Format branch info
    let branch_str = session
        .git_branch
        .as_ref()
        .map(|b| format!(" [{}]", b))
        .unwrap_or_default();

    format!(
        "{}{} {} {} {} {}",
        display_name.bright_white(),
        branch_str.green(),
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

    // Show title hierarchy (Claude Code style)
    println!(
        "  {} {}",
        "Title:".dimmed(),
        session.display_title().bright_white()
    );

    if let Some(ref custom_title) = session.custom_title {
        println!("  {} {}", "Custom Title:".dimmed(), custom_title.bright_white());
    }

    if let Some(ref summary) = session.summary {
        println!("  {} {}", "Summary:".dimmed(), summary.italic());
    }

    if let Some(ref first_prompt) = session.first_prompt {
        println!("  {} {}", "First Prompt:".dimmed(), first_prompt.dimmed());
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

    println!("  {} {}", "State:".dimmed(), format_state(&session.state));

    // Show sidechain info if applicable
    if session.is_sidechain {
        println!("  {} {}", "Type:".dimmed(), "Sidechain (branched)".magenta());
        if let Some(ref parent_id) = session.parent_session_id {
            println!("  {} {}", "Parent:".dimmed(), parent_id.dimmed());
        }
    }

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
