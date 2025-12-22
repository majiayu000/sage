//! Session types for persistence and recovery
//!
//! This module defines the core types used in the session system,
//! including session state, messages, and metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

// =============================================================================
// Re-exports from submodules
// =============================================================================

// Re-export enhanced message types
pub use super::enhanced::{
    EnhancedMessage, EnhancedMessageType, EnhancedTokenUsage, EnhancedToolCall,
    EnhancedToolResult, MessageContent, SessionContext, ThinkingLevel, ThinkingMetadata,
    TodoItem, TodoStatus,
};

// Re-export file tracking types
pub use super::file_tracking::{
    FileBackupInfo, FileHistorySnapshot, TrackedFileState, TrackedFilesSnapshot,
};

// =============================================================================
// Core Session Types
// =============================================================================

/// Unique session identifier
pub type SessionId = String;

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is actively being used
    Active,
    /// Session is paused/suspended
    Paused,
    /// Session completed successfully
    Completed,
    /// Session failed with an error
    Failed,
    /// Session was cancelled by user
    Cancelled,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::Active => write!(f, "active"),
            SessionState::Paused => write!(f, "paused"),
            SessionState::Completed => write!(f, "completed"),
            SessionState::Failed => write!(f, "failed"),
            SessionState::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Active
    }
}

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// System message (instructions)
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool use message
    Tool,
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

impl Default for MessageRole {
    fn default() -> Self {
        MessageRole::User
    }
}

/// A tool call made during the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToolCall {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: HashMap<String, Value>,
    /// Timestamp of the call
    pub timestamp: DateTime<Utc>,
}

impl SessionToolCall {
    /// Create a new tool call
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: HashMap<String, Value>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
            timestamp: Utc::now(),
        }
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToolResult {
    /// ID of the tool call this result is for
    pub tool_call_id: String,
    /// Tool name
    pub tool_name: String,
    /// Result content
    pub content: String,
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl SessionToolResult {
    /// Create a successful tool result
    pub fn success(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            content: content.into(),
            success: true,
            error: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a failed tool result
    pub fn failure(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            content: String::new(),
            success: false,
            error: Some(error.into()),
            timestamp: Utc::now(),
        }
    }
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Tool calls (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<SessionToolCall>>,
    /// Tool results (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<SessionToolResult>>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl ConversationMessage {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message with tool calls
    pub fn assistant_with_tools(
        content: impl Into<String>,
        tool_calls: Vec<SessionToolCall>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_results: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new tool result message
    pub fn tool_results(results: Vec<SessionToolResult>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: String::new(),
            tool_calls: None,
            tool_results: Some(results),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens used
    pub output_tokens: u64,
    /// Cache read tokens
    pub cache_read_tokens: u64,
    /// Cache write tokens
    pub cache_write_tokens: u64,
    /// Total cost estimate (in USD)
    pub cost_estimate: f64,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new() -> Self {
        Self::default()
    }

    /// Add usage from another TokenUsage
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.cost_estimate += other.cost_estimate;
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

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
            message_count: session.messages.len(),
            state: session.state,
            model: session.model.clone(),
        }
    }
}

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
    fn test_conversation_message_user() {
        let msg = ConversationMessage::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_conversation_message_assistant() {
        let msg = ConversationMessage::assistant("Hello");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_conversation_message_system() {
        let msg = ConversationMessage::system("You are a helpful assistant");
        assert_eq!(msg.role, MessageRole::System);
    }

    #[test]
    fn test_token_usage() {
        let mut usage1 = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 10,
            cache_write_tokens: 5,
            cost_estimate: 0.01,
        };

        let usage2 = TokenUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_tokens: 20,
            cache_write_tokens: 10,
            cost_estimate: 0.02,
        };

        usage1.add(&usage2);
        assert_eq!(usage1.input_tokens, 300);
        assert_eq!(usage1.output_tokens, 150);
        assert_eq!(usage1.total_tokens(), 450);
    }

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
    fn test_session_tool_call() {
        let tool_call = SessionToolCall::new("call-1", "bash", HashMap::new());
        assert_eq!(tool_call.id, "call-1");
        assert_eq!(tool_call.name, "bash");
    }

    #[test]
    fn test_session_tool_result() {
        let result = SessionToolResult::success("call-1", "bash", "output");
        assert!(result.success);
        assert_eq!(result.content, "output");

        let result = SessionToolResult::failure("call-2", "bash", "error");
        assert!(!result.success);
        assert_eq!(result.error, Some("error".to_string()));
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

    #[test]
    fn test_session_serialization() {
        let session = Session::new(PathBuf::from("/tmp")).with_name("Test");

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.name, session.name);
    }

    #[test]
    fn test_session_state_display() {
        assert_eq!(format!("{}", SessionState::Active), "active");
        assert_eq!(format!("{}", SessionState::Paused), "paused");
        assert_eq!(format!("{}", SessionState::Completed), "completed");
        assert_eq!(format!("{}", SessionState::Failed), "failed");
        assert_eq!(format!("{}", SessionState::Cancelled), "cancelled");
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(format!("{}", MessageRole::System), "system");
        assert_eq!(format!("{}", MessageRole::User), "user");
        assert_eq!(format!("{}", MessageRole::Assistant), "assistant");
        assert_eq!(format!("{}", MessageRole::Tool), "tool");
    }
}
