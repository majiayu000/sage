//! Utility functions for the unified command

use crate::console::CliConsole;
use crate::ui::nerd_console::SessionInfo;
use crate::ui::NerdConsole;
use sage_core::error::{SageError, SageResult};
use sage_core::session::JsonlSessionStorage;
use std::sync::Arc;

/// Load task description from argument (might be a file path)
pub async fn load_task_from_arg(task: &str, console: &CliConsole) -> SageResult<String> {
    if let Ok(task_path) = std::path::Path::new(task).canonicalize() {
        if task_path.is_file() {
            console.info(&format!("Loading task from file: {}", task_path.display()));
            return tokio::fs::read_to_string(&task_path)
                .await
                .map_err(|e| SageError::config(format!("Failed to read task file: {e}")));
        }
    }
    Ok(task.to_string())
}

/// Get current git branch
pub fn get_git_branch(working_dir: &std::path::Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(working_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Show recent activity with Nerd Font style
pub async fn show_recent_activity_nerd(
    nerd: &NerdConsole,
    storage: &Arc<JsonlSessionStorage>,
) {
    let sessions = match storage.list_sessions().await {
        Ok(s) => s,
        Err(_) => return,
    };

    if sessions.is_empty() {
        return;
    }

    let session_infos: Vec<SessionInfo> = sessions
        .iter()
        .take(5)
        .map(|s| SessionInfo {
            title: s.display_title().to_string(),
            time_ago: format_time_ago(&s.updated_at),
            message_count: s.message_count,
        })
        .collect();

    nerd.print_sessions_tree(&session_infos);
}

/// Format time difference as human-readable string
pub fn format_time_ago(dt: &chrono::DateTime<chrono::Utc>) -> String {
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
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_chars {
        let truncated: String = chars[..max_chars.saturating_sub(3)].iter().collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
