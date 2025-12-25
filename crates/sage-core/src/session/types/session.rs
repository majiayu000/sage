//! Session type and implementation
//!
//! This module contains the core Session struct representing a conversation
//! with context, along with all its methods.

use super::base::{SessionId, SessionState, TokenUsage};
use super::super::conversation::ConversationMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// Session
// =============================================================================

/// A session representing a conversation with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: SessionId,
    /// Session name/title (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Working directory for this session
    pub working_directory: PathBuf,
    /// Conversation messages
    pub messages: Vec<ConversationMessage>,
    /// Token usage statistics
    pub token_usage: TokenUsage,
    /// Current session state
    pub state: SessionState,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Model used for this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl Session {
    /// Create a new session
    pub fn new(working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            created_at: now,
            updated_at: now,
            working_directory,
            messages: Vec::new(),
            token_usage: TokenUsage::default(),
            state: SessionState::Active,
            error: None,
            model: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new session with a specific ID
    pub fn with_id(id: impl Into<String>, working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: None,
            created_at: now,
            updated_at: now,
            working_directory,
            messages: Vec::new(),
            token_usage: TokenUsage::default(),
            state: SessionState::Active,
            error: None,
            model: None,
            metadata: HashMap::new(),
        }
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

    /// Add a message to the session
    pub fn add_message(&mut self, message: ConversationMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Update token usage
    pub fn update_token_usage(&mut self, usage: &TokenUsage) {
        self.token_usage.add(usage);
        self.updated_at = Utc::now();
    }

    /// Set the session state
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    /// Mark the session as completed
    pub fn complete(&mut self) {
        self.state = SessionState::Completed;
        self.updated_at = Utc::now();
    }

    /// Mark the session as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.state = SessionState::Failed;
        self.error = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// Pause the session
    pub fn pause(&mut self) {
        self.state = SessionState::Paused;
        self.updated_at = Utc::now();
    }

    /// Resume the session
    pub fn resume(&mut self) {
        self.state = SessionState::Active;
        self.updated_at = Utc::now();
    }

    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if the session is active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Check if the session is finished (completed, failed, or cancelled)
    pub fn is_finished(&self) -> bool {
        matches!(
            self.state,
            SessionState::Completed | SessionState::Failed | SessionState::Cancelled
        )
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
        self.updated_at = Utc::now();
    }

    /// Get the duration of the session
    pub fn duration(&self) -> chrono::Duration {
        self.updated_at - self.created_at
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new(PathBuf::from("/tmp"));
        assert!(!session.id.is_empty());
        assert!(session.messages.is_empty());
        assert_eq!(session.state, SessionState::Active);
    }

    #[test]
    fn test_session_with_id() {
        let session = Session::with_id("test-id", PathBuf::from("/tmp"));
        assert_eq!(session.id, "test-id");
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new(PathBuf::from("/tmp"));
        session.add_message(ConversationMessage::user("Hello"));
        session.add_message(ConversationMessage::assistant("Hi there!"));
        assert_eq!(session.message_count(), 2);
    }

    #[test]
    fn test_session_state_transitions() {
        let mut session = Session::new(PathBuf::from("/tmp"));
        assert!(session.is_active());
        assert!(!session.is_finished());

        session.pause();
        assert_eq!(session.state, SessionState::Paused);
        assert!(!session.is_active());

        session.resume();
        assert!(session.is_active());

        session.complete();
        assert!(session.is_finished());
        assert_eq!(session.state, SessionState::Completed);
    }

    #[test]
    fn test_session_fail() {
        let mut session = Session::new(PathBuf::from("/tmp"));
        session.fail("Something went wrong");
        assert_eq!(session.state, SessionState::Failed);
        assert_eq!(session.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new(PathBuf::from("/tmp")).with_name("Test");

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.name, session.name);
    }
}
