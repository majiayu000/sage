//! Session data structures
//!
//! Defines the core types for session management:
//! - Session: Full session with messages and metadata
//! - SessionMetadata: Lightweight session info for listing
//! - Message: Individual conversation messages
//! - Summary: Session summaries for context compression

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Session metadata for listing and filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Unique session identifier
    pub id: String,

    /// Session title (auto-generated or user-defined)
    pub title: String,

    /// Creation timestamp
    pub created: DateTime<Utc>,

    /// Last modification timestamp
    pub modified: DateTime<Utc>,

    /// Number of messages in the session
    pub message_count: usize,

    /// Associated Git branch (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Associated project path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<PathBuf>,

    /// Whether this is a sidechain/branch session
    #[serde(default)]
    pub is_sidechain: bool,

    /// Total tokens used in this session
    #[serde(default)]
    pub total_tokens: usize,
}

impl SessionMetadata {
    /// Create new session metadata with generated ID
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            created: now,
            modified: now,
            message_count: 0,
            git_branch: None,
            project_path: None,
            is_sidechain: false,
            total_tokens: 0,
        }
    }

    /// Create metadata with project context
    pub fn with_project(mut self, path: PathBuf, branch: Option<String>) -> Self {
        self.project_path = Some(path);
        self.git_branch = branch;
        self
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.modified = Utc::now();
    }
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant response
    Assistant,
    /// System prompt
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,

    /// Tool arguments
    pub arguments: serde_json::Value,

    /// Tool result (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,

    /// Whether the tool call succeeded
    #[serde(default)]
    pub success: bool,
}

/// Individual conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: Role,

    /// Message content
    pub content: String,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Tool calls made in this message (assistant only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Token count for this message
    #[serde(default)]
    pub tokens: usize,
}

impl Message {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            timestamp: Utc::now(),
            tool_calls: None,
            tokens: 0,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tool_calls: None,
            tokens: 0,
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            timestamp: Utc::now(),
            tool_calls: None,
            tokens: 0,
        }
    }

    /// Add tool calls to the message
    pub fn with_tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(calls);
        self
    }

    /// Set token count
    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens = tokens;
        self
    }
}

/// Session summary for context compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Summary content
    pub content: String,

    /// Range of messages summarized (start index)
    pub start_index: usize,

    /// Range of messages summarized (end index)
    pub end_index: usize,

    /// When the summary was created
    pub created: DateTime<Utc>,
}

impl Summary {
    /// Create a new summary
    pub fn new(content: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            content: content.into(),
            start_index: start,
            end_index: end,
            created: Utc::now(),
        }
    }
}

/// Complete session with messages and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session metadata
    pub metadata: SessionMetadata,

    /// Conversation messages
    pub messages: Vec<Message>,

    /// Session summaries (for long conversations)
    #[serde(default)]
    pub summaries: Vec<Summary>,
}

impl Session {
    /// Create a new empty session
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            metadata: SessionMetadata::new(title),
            messages: Vec::new(),
            summaries: Vec::new(),
        }
    }

    /// Create a session with project context
    pub fn with_project(title: impl Into<String>, path: PathBuf, branch: Option<String>) -> Self {
        Self {
            metadata: SessionMetadata::new(title).with_project(path, branch),
            messages: Vec::new(),
            summaries: Vec::new(),
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: Message) {
        self.metadata.total_tokens += message.tokens;
        self.messages.push(message);
        self.metadata.message_count = self.messages.len();
        self.metadata.touch();
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.add_message(Message::user(content));
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.add_message(Message::assistant(content));
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// Get the session title
    pub fn title(&self) -> &str {
        &self.metadata.title
    }

    /// Set the session title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.metadata.title = title.into();
        self.metadata.touch();
    }

    /// Get all messages as a slice
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get the last N messages
    pub fn last_messages(&self, n: usize) -> &[Message] {
        let len = self.messages.len();
        if n >= len {
            &self.messages
        } else {
            &self.messages[len - n..]
        }
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get message count
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Add a summary
    pub fn add_summary(&mut self, summary: Summary) {
        self.summaries.push(summary);
        self.metadata.touch();
    }

    /// Generate an auto-title from the first user message
    pub fn auto_title(&mut self) {
        if let Some(first_user_msg) = self.messages.iter().find(|m| m.role == Role::User) {
            let title = first_user_msg
                .content
                .chars()
                .take(50)
                .collect::<String>()
                .trim()
                .to_string();

            if !title.is_empty() {
                self.metadata.title = if title.len() < first_user_msg.content.len() {
                    format!("{}...", title)
                } else {
                    title
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("Test Session");
        assert_eq!(session.title(), "Test Session");
        assert!(session.is_empty());
        assert_eq!(session.len(), 0);
    }

    #[test]
    fn test_add_messages() {
        let mut session = Session::new("Test");
        session.add_user_message("Hello");
        session.add_assistant_message("Hi there!");

        assert_eq!(session.len(), 2);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(session.messages[1].role, Role::Assistant);
    }

    #[test]
    fn test_auto_title() {
        let mut session = Session::new("Untitled");
        session.add_user_message("How do I implement a binary search tree in Rust?");
        session.auto_title();

        assert!(session.title().starts_with("How do I implement"));
    }

    #[test]
    fn test_last_messages() {
        let mut session = Session::new("Test");
        for i in 0..10 {
            session.add_user_message(format!("Message {}", i));
        }

        let last_3 = session.last_messages(3);
        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].content, "Message 7");
    }
}
