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

// =============================================================================
// Enhanced Message Types (Claude Code-inspired)
// =============================================================================

/// Enhanced message type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnhancedMessageType {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool result message
    ToolResult,
    /// System message
    System,
    /// File history snapshot
    FileHistorySnapshot,
}

impl fmt::Display for EnhancedMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::ToolResult => write!(f, "tool_result"),
            Self::System => write!(f, "system"),
            Self::FileHistorySnapshot => write!(f, "file_history_snapshot"),
        }
    }
}

/// Session context embedded in each message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Current working directory
    pub cwd: PathBuf,

    /// Current git branch (if in git repo)
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Platform (macos, linux, windows)
    pub platform: String,

    /// User type (external, internal)
    #[serde(rename = "userType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
}

impl SessionContext {
    /// Create a new session context
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            git_branch: None,
            platform: std::env::consts::OS.to_string(),
            user_type: Some("external".to_string()),
        }
    }

    /// Create context with git branch
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set user type
    pub fn with_user_type(mut self, user_type: impl Into<String>) -> Self {
        self.user_type = Some(user_type.into());
        self
    }

    /// Detect git branch from cwd
    pub fn detect_git_branch(&mut self) {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.cwd)
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    self.git_branch = Some(branch);
                }
            }
        }
    }
}

/// Thinking level for extended thinking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    /// No extended thinking
    None,
    /// Low level thinking
    Low,
    /// Medium level thinking
    #[default]
    Medium,
    /// High level thinking (ultrathink)
    High,
}

impl fmt::Display for ThinkingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

/// Thinking metadata for extended thinking control
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThinkingMetadata {
    /// Thinking level
    pub level: ThinkingLevel,

    /// Whether extended thinking is disabled
    pub disabled: bool,

    /// Triggers that activated thinking
    #[serde(default)]
    pub triggers: Vec<String>,
}

impl ThinkingMetadata {
    /// Create default thinking metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific level
    pub fn with_level(level: ThinkingLevel) -> Self {
        Self {
            level,
            disabled: false,
            triggers: Vec::new(),
        }
    }

    /// Disable thinking
    pub fn disabled() -> Self {
        Self {
            level: ThinkingLevel::None,
            disabled: true,
            triggers: Vec::new(),
        }
    }

    /// Add a trigger
    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.triggers.push(trigger.into());
        self
    }
}

/// Todo item for task tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Task content (imperative form)
    pub content: String,

    /// Task status
    pub status: TodoStatus,

    /// Active form (present continuous)
    #[serde(rename = "activeForm")]
    pub active_form: String,
}

/// Todo status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Task not yet started
    Pending,
    /// Task in progress
    InProgress,
    /// Task completed
    Completed,
}

impl fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
        }
    }
}

/// Enhanced message with full context (Claude Code-style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMessage {
    /// Message type
    #[serde(rename = "type")]
    pub message_type: EnhancedMessageType,

    /// Unique message identifier
    pub uuid: String,

    /// Parent message UUID (for message chains)
    #[serde(rename = "parentUuid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_uuid: Option<String>,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,

    /// Sage Agent version
    pub version: String,

    /// Session context
    pub context: SessionContext,

    /// Message content
    pub message: MessageContent,

    /// Token usage (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<EnhancedTokenUsage>,

    /// Thinking metadata
    #[serde(rename = "thinkingMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_metadata: Option<ThinkingMetadata>,

    /// Todo list snapshot
    #[serde(default)]
    pub todos: Vec<TodoItem>,

    /// Whether this is a sidechain (branch)
    #[serde(rename = "isSidechain")]
    #[serde(default)]
    pub is_sidechain: bool,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    /// Role
    pub role: String,

    /// Text content
    pub content: String,

    /// Tool calls (for assistant messages)
    #[serde(rename = "toolCalls")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<EnhancedToolCall>>,

    /// Tool results (for tool_result messages)
    #[serde(rename = "toolResults")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<EnhancedToolResult>>,
}

/// Enhanced tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolCall {
    /// Tool call ID
    pub id: String,

    /// Tool name
    pub name: String,

    /// Tool arguments
    pub arguments: Value,
}

/// Enhanced tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolResult {
    /// Tool call ID this result is for
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,

    /// Tool name
    #[serde(rename = "toolName")]
    pub tool_name: String,

    /// Result content
    pub content: String,

    /// Whether execution succeeded
    pub success: bool,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Enhanced token usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnhancedTokenUsage {
    /// Input tokens used
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,

    /// Output tokens used
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,

    /// Cache read tokens
    #[serde(rename = "cacheReadTokens")]
    #[serde(default)]
    pub cache_read_tokens: u64,

    /// Cache write tokens
    #[serde(rename = "cacheWriteTokens")]
    #[serde(default)]
    pub cache_write_tokens: u64,
}

impl EnhancedMessage {
    /// Create a new user message
    pub fn user(
        content: impl Into<String>,
        session_id: impl Into<String>,
        context: SessionContext,
    ) -> Self {
        Self {
            message_type: EnhancedMessageType::User,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid: None,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context,
            message: MessageContent {
                role: "user".to_string(),
                content: content.into(),
                tool_calls: None,
                tool_results: None,
            },
            usage: None,
            thinking_metadata: None,
            todos: Vec::new(),
            is_sidechain: false,
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(
        content: impl Into<String>,
        session_id: impl Into<String>,
        context: SessionContext,
        parent_uuid: Option<String>,
    ) -> Self {
        Self {
            message_type: EnhancedMessageType::Assistant,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context,
            message: MessageContent {
                role: "assistant".to_string(),
                content: content.into(),
                tool_calls: None,
                tool_results: None,
            },
            usage: None,
            thinking_metadata: None,
            todos: Vec::new(),
            is_sidechain: false,
            metadata: HashMap::new(),
        }
    }

    /// Set parent UUID
    pub fn with_parent(mut self, parent_uuid: impl Into<String>) -> Self {
        self.parent_uuid = Some(parent_uuid.into());
        self
    }

    /// Set tool calls
    pub fn with_tool_calls(mut self, tool_calls: Vec<EnhancedToolCall>) -> Self {
        self.message.tool_calls = Some(tool_calls);
        self
    }

    /// Set token usage
    pub fn with_usage(mut self, usage: EnhancedTokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Set thinking metadata
    pub fn with_thinking(mut self, thinking: ThinkingMetadata) -> Self {
        self.thinking_metadata = Some(thinking);
        self
    }

    /// Set todos
    pub fn with_todos(mut self, todos: Vec<TodoItem>) -> Self {
        self.todos = todos;
        self
    }

    /// Mark as sidechain
    pub fn as_sidechain(mut self) -> Self {
        self.is_sidechain = true;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get UUID
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// Check if this is a user message
    pub fn is_user(&self) -> bool {
        self.message_type == EnhancedMessageType::User
    }

    /// Check if this is an assistant message
    pub fn is_assistant(&self) -> bool {
        self.message_type == EnhancedMessageType::Assistant
    }
}

/// File history snapshot linked to a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistorySnapshot {
    /// Snapshot type
    #[serde(rename = "type")]
    pub snapshot_type: String,

    /// Associated message ID
    #[serde(rename = "messageId")]
    pub message_id: String,

    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// Whether this is an update to existing snapshot
    #[serde(rename = "isSnapshotUpdate")]
    pub is_snapshot_update: bool,

    /// Actual snapshot data
    pub snapshot: TrackedFilesSnapshot,
}

/// Tracked files snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFilesSnapshot {
    /// Tracked files with their state
    #[serde(rename = "trackedFiles")]
    #[serde(default)]
    pub tracked_files: HashMap<String, TrackedFileState>,

    /// File backups for undo
    #[serde(rename = "fileBackups")]
    #[serde(default)]
    pub file_backups: HashMap<String, FileBackupInfo>,
}

/// State of a tracked file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFileState {
    /// Original content (None if file didn't exist)
    #[serde(rename = "originalContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_content: Option<String>,

    /// Content hash
    #[serde(rename = "contentHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    /// File size in bytes
    pub size: u64,

    /// File state (created, modified, deleted, unchanged)
    pub state: String,
}

/// File backup information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBackupInfo {
    /// Path to backup file
    #[serde(rename = "backupPath")]
    pub backup_path: String,

    /// Original content hash
    #[serde(rename = "originalHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hash: Option<String>,
}

impl FileHistorySnapshot {
    /// Create a new file history snapshot
    pub fn new(message_id: impl Into<String>) -> Self {
        Self {
            snapshot_type: "file_history_snapshot".to_string(),
            message_id: message_id.into(),
            timestamp: Utc::now(),
            is_snapshot_update: false,
            snapshot: TrackedFilesSnapshot {
                tracked_files: HashMap::new(),
                file_backups: HashMap::new(),
            },
        }
    }

    /// Create an update snapshot
    pub fn update(message_id: impl Into<String>) -> Self {
        Self {
            snapshot_type: "file_history_snapshot".to_string(),
            message_id: message_id.into(),
            timestamp: Utc::now(),
            is_snapshot_update: true,
            snapshot: TrackedFilesSnapshot {
                tracked_files: HashMap::new(),
                file_backups: HashMap::new(),
            },
        }
    }

    /// Add tracked file
    pub fn with_file(mut self, path: impl Into<String>, state: TrackedFileState) -> Self {
        self.snapshot.tracked_files.insert(path.into(), state);
        self
    }

    /// Add file backup
    pub fn with_backup(mut self, path: impl Into<String>, backup: FileBackupInfo) -> Self {
        self.snapshot.file_backups.insert(path.into(), backup);
        self
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
