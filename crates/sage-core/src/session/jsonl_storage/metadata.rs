//! Session metadata type and operations
//!
//! Following Claude Code's session design pattern with:
//! - first_prompt: Preview of first user message
//! - summary: Auto-generated conversation summary
//! - is_sidechain: Branched conversation flag

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::super::types::SessionId;

/// Truncate a string to a maximum number of characters (UTF-8 safe)
fn truncate_string(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_chars {
        let truncated: String = chars[..max_chars.saturating_sub(3)].iter().collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Session metadata stored in metadata.json
///
/// Follows Claude Code's session design for compatibility and feature parity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session ID
    pub id: SessionId,

    /// Custom session title (user-defined)
    #[serde(rename = "customTitle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,

    /// Session name (auto-generated or legacy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// First user message preview (for quick display in session list)
    #[serde(rename = "firstPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,

    /// Auto-generated conversation summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Working directory
    #[serde(rename = "workingDirectory")]
    pub working_directory: PathBuf,

    /// Git branch at session start
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Sage Agent version
    pub version: String,

    /// Total message count
    #[serde(rename = "messageCount")]
    pub message_count: usize,

    /// Session state (active, completed, failed)
    pub state: String,

    /// Whether this is a sidechain (branched) session
    #[serde(rename = "isSidechain")]
    #[serde(default)]
    pub is_sidechain: bool,

    /// Parent session ID (for sidechain sessions)
    #[serde(rename = "parentSessionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionMetadata {
    /// Create new session metadata
    pub fn new(id: impl Into<String>, working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            custom_title: None,
            name: None,
            first_prompt: None,
            summary: None,
            created_at: now,
            updated_at: now,
            working_directory,
            git_branch: None,
            model: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message_count: 0,
            state: "active".to_string(),
            is_sidechain: false,
            parent_session_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set custom title (user-defined)
    pub fn with_custom_title(mut self, title: impl Into<String>) -> Self {
        self.custom_title = Some(title.into());
        self
    }

    /// Set session name (legacy/auto-generated)
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set first prompt preview
    pub fn with_first_prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        // Truncate to 100 chars for preview (safe for UTF-8)
        self.first_prompt = Some(truncate_string(&prompt, 100));
        self
    }

    /// Set summary
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set git branch
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Mark as sidechain session
    pub fn as_sidechain(mut self, parent_id: impl Into<String>) -> Self {
        self.is_sidechain = true;
        self.parent_session_id = Some(parent_id.into());
        self
    }

    /// Update message count
    pub fn update_message_count(&mut self, count: usize) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }

    /// Update first prompt (only if not set)
    pub fn set_first_prompt_if_empty(&mut self, prompt: &str) {
        if self.first_prompt.is_none() {
            self.first_prompt = Some(truncate_string(prompt, 100));
            self.updated_at = Utc::now();
        }
    }

    /// Update summary
    pub fn set_summary(&mut self, summary: impl Into<String>) {
        self.summary = Some(summary.into());
        self.updated_at = Utc::now();
    }

    /// Set custom title
    pub fn set_custom_title(&mut self, title: impl Into<String>) {
        self.custom_title = Some(title.into());
        self.updated_at = Utc::now();
    }

    /// Set state
    pub fn set_state(&mut self, state: impl Into<String>) {
        self.state = state.into();
        self.updated_at = Utc::now();
    }

    /// Get display title (custom_title > summary > first_prompt > name > id)
    pub fn display_title(&self) -> &str {
        self.custom_title
            .as_deref()
            .or(self.summary.as_deref())
            .or(self.first_prompt.as_deref())
            .or(self.name.as_deref())
            .unwrap_or(&self.id)
    }
}
