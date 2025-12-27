//! Session metadata and configuration types
//!
//! This module contains types for session summaries and configuration.

use super::base::{SessionId, SessionState};
use super::session::Session;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// Session Summary
// =============================================================================

/// Summary of a session for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session ID
    pub id: SessionId,
    /// Session name
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Working directory
    pub working_directory: PathBuf,
    /// Associated Git branch
    pub git_branch: Option<String>,
    /// Number of messages
    pub message_count: usize,
    /// Session state
    pub state: SessionState,
    /// Model used
    pub model: Option<String>,
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            name: session.name.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
            working_directory: session.working_directory.clone(),
            git_branch: session.git_branch.clone(),
            message_count: session.messages.len(),
            state: session.state,
            model: session.model.clone(),
        }
    }
}

// =============================================================================
// Session Configuration
// =============================================================================

/// Configuration for creating a new session
#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Session name
    pub name: Option<String>,
    /// Model to use
    pub model: Option<String>,
    /// Initial system prompt
    pub system_prompt: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl SessionConfig {
    /// Create a new session config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the working directory
    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    /// Set the session name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::conversation::ConversationMessage;

    #[test]
    fn test_session_summary() {
        let mut session = Session::new(PathBuf::from("/tmp"));
        session.name = Some("Test Session".to_string());
        session.model = Some("claude-3".to_string());
        session.add_message(ConversationMessage::user("Hello"));

        let summary: SessionSummary = (&session).into();
        assert_eq!(summary.id, session.id);
        assert_eq!(summary.name, Some("Test Session".to_string()));
        assert_eq!(summary.message_count, 1);
    }

    #[test]
    fn test_session_config() {
        let config = SessionConfig::new()
            .with_working_directory("/tmp")
            .with_name("Test")
            .with_model("claude-3")
            .with_system_prompt("You are helpful");

        assert_eq!(config.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(config.name, Some("Test".to_string()));
        assert_eq!(config.model, Some("claude-3".to_string()));
        assert_eq!(config.system_prompt, Some("You are helpful".to_string()));
    }
}
