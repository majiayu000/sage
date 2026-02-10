//! Unified Session Data Model
//!
//! This module provides a unified data model for session management,
//! following Claude Code's design patterns:
//! - uuid + parentUuid message chains
//! - Sidechain branching support
//! - Real-time JSONL persistence
//! - SessionRecord for append-only storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

// Import SessionState from base module (canonical definition)
pub use super::base::SessionState;

// Import canonical context types from enhanced module to avoid duplication
pub use super::super::enhanced::context::{
    SessionContext, ThinkingLevel, ThinkingMetadata, TodoItem, TodoStatus,
};

// ============================================================================
// Type Aliases
// ============================================================================

/// Unique session identifier
pub type SessionId = String;

/// Unique message identifier (UUID)
pub type MessageId = String;

/// Branch identifier for sidechains
pub type BranchId = String;

// ============================================================================
// Session Header (metadata.json)
// ============================================================================

/// Session metadata stored in metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    /// Unique session ID
    pub id: SessionId,

    /// User-defined custom title
    #[serde(rename = "customTitle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,

    /// Session name (auto-generated or user-defined)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Preview of first user message
    #[serde(rename = "firstPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,

    /// Preview of last user message
    #[serde(rename = "lastPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_prompt: Option<String>,

    /// Auto-generated conversation summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Working directory for this session
    #[serde(rename = "workingDirectory")]
    pub working_directory: PathBuf,

    /// Git branch (if in git repo)
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Model used for this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Sage version
    pub version: String,

    /// Total message count
    #[serde(rename = "messageCount")]
    pub message_count: usize,

    /// Session state
    pub state: SessionState,

    /// Token usage statistics
    #[serde(rename = "tokenUsage")]
    #[serde(default)]
    pub token_usage: UnifiedTokenUsage,

    /// Whether this is a sidechain (branched session)
    #[serde(rename = "isSidechain")]
    #[serde(default)]
    pub is_sidechain: bool,

    /// Parent session ID (for sidechains)
    #[serde(rename = "parentSessionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<SessionId>,

    /// Root message ID where sidechain branched
    #[serde(rename = "sidechainRootMessageId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidechain_root_message_id: Option<MessageId>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl SessionHeader {
    /// Create a new session header
    pub fn new(id: impl Into<String>, working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            custom_title: None,
            name: None,
            first_prompt: None,
            last_prompt: None,
            summary: None,
            created_at: now,
            updated_at: now,
            working_directory,
            git_branch: None,
            model: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message_count: 0,
            state: SessionState::Active,
            token_usage: UnifiedTokenUsage::default(),
            is_sidechain: false,
            parent_session_id: None,
            sidechain_root_message_id: None,
            metadata: HashMap::new(),
        }
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

    /// Mark as sidechain
    pub fn as_sidechain(
        mut self,
        parent_session_id: impl Into<String>,
        root_message_id: impl Into<String>,
    ) -> Self {
        self.is_sidechain = true;
        self.parent_session_id = Some(parent_session_id.into());
        self.sidechain_root_message_id = Some(root_message_id.into());
        self
    }
}

// ============================================================================
// Session (In-Memory Aggregate)
// ============================================================================

/// Session aggregate view (in-memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session metadata
    pub header: SessionHeader,

    /// Messages (loaded from JSONL)
    #[serde(default)]
    pub messages: Vec<SessionMessage>,

    /// File history snapshots
    #[serde(default)]
    pub snapshots: Vec<FileHistorySnapshot>,
}

impl Session {
    /// Create a new session
    pub fn new(working_directory: PathBuf) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            header: SessionHeader::new(id, working_directory),
            messages: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    /// Create with specific ID
    pub fn with_id(id: impl Into<String>, working_directory: PathBuf) -> Self {
        Self {
            header: SessionHeader::new(id, working_directory),
            messages: Vec::new(),
            snapshots: Vec::new(),
        }
    }
}

// ============================================================================
// JSONL Record (Append-Only Persistence)
// ============================================================================

/// JSONL record for real-time persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Sequence number (monotonically increasing)
    pub seq: u64,

    /// Record timestamp
    pub timestamp: DateTime<Utc>,

    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,

    /// Record payload (not flattened to avoid field conflicts)
    pub payload: SessionRecordPayload,
}

/// Record payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "recordType", rename_all = "snake_case")]
pub enum SessionRecordPayload {
    /// Message record
    Message(SessionMessage),
    /// File snapshot record
    Snapshot(FileHistorySnapshot),
    /// Metadata update record
    MetadataPatch(SessionMetadataPatch),
}

/// Metadata patch for incremental updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadataPatch {
    /// Update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Updated message count
    #[serde(rename = "messageCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<usize>,

    /// Updated last prompt
    #[serde(rename = "lastPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_prompt: Option<String>,

    /// Updated summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Updated custom title
    #[serde(rename = "customTitle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,

    /// Updated state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<SessionState>,

    /// Updated token usage
    #[serde(rename = "tokenUsage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<UnifiedTokenUsage>,
}

// ============================================================================
// Session Message (Unified Message Model)
// ============================================================================

/// Unified message model (replaces ConversationMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message type
    #[serde(rename = "type")]
    pub message_type: SessionMessageType,

    /// Unique message ID
    pub uuid: MessageId,

    /// Parent message ID (for message chains)
    #[serde(rename = "parentUuid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_uuid: Option<MessageId>,

    /// Branch ID (for sidechains)
    #[serde(rename = "branchId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<BranchId>,

    /// Parent message ID in branch
    #[serde(rename = "branchParentUuid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_parent_uuid: Option<MessageId>,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,

    /// Sage version
    pub version: String,

    /// Session context
    pub context: SessionContext,

    /// Message content
    pub message: MessageContent,

    /// Token usage (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UnifiedTokenUsage>,

    /// Thinking metadata
    #[serde(rename = "thinkingMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_metadata: Option<ThinkingMetadata>,

    /// Todo list snapshot
    #[serde(default)]
    pub todos: Vec<TodoItem>,

    /// Whether this is a sidechain message
    #[serde(rename = "isSidechain")]
    #[serde(default)]
    pub is_sidechain: bool,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl SessionMessage {
    /// Create a new user message
    pub fn user(
        content: impl Into<String>,
        session_id: impl Into<String>,
        context: SessionContext,
    ) -> Self {
        Self {
            message_type: SessionMessageType::User,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid: None,
            branch_id: None,
            branch_parent_uuid: None,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context,
            message: MessageContent {
                role: UnifiedMessageRole::User,
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
            message_type: SessionMessageType::Assistant,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid,
            branch_id: None,
            branch_parent_uuid: None,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context,
            message: MessageContent {
                role: UnifiedMessageRole::Assistant,
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

    /// Create a tool result message
    pub fn tool_result(
        results: Vec<UnifiedToolResult>,
        session_id: impl Into<String>,
        context: SessionContext,
        parent_uuid: Option<String>,
    ) -> Self {
        Self {
            message_type: SessionMessageType::ToolResult,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid,
            branch_id: None,
            branch_parent_uuid: None,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context,
            message: MessageContent {
                role: UnifiedMessageRole::Tool,
                content: String::new(),
                tool_calls: None,
                tool_results: Some(results),
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
    pub fn with_tool_calls(mut self, tool_calls: Vec<UnifiedToolCall>) -> Self {
        self.message.tool_calls = Some(tool_calls);
        self
    }

    /// Set token usage
    pub fn with_usage(mut self, usage: UnifiedTokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Set thinking metadata
    pub fn with_thinking(mut self, thinking: ThinkingMetadata) -> Self {
        self.thinking_metadata = Some(thinking);
        self
    }

    /// Mark as sidechain
    pub fn as_sidechain(mut self, branch_id: impl Into<String>) -> Self {
        self.is_sidechain = true;
        self.branch_id = Some(branch_id.into());
        self
    }

    /// Set todos
    pub fn with_todos(mut self, todos: Vec<TodoItem>) -> Self {
        self.todos = todos;
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
        self.message_type == SessionMessageType::User
    }

    /// Check if this is an assistant message
    pub fn is_assistant(&self) -> bool {
        self.message_type == SessionMessageType::Assistant
    }

    /// Check if this is an error message
    pub fn is_error(&self) -> bool {
        self.message_type == SessionMessageType::Error
    }

    /// Create a new error message
    ///
    /// Records execution errors, API failures, etc. for debugging and session review.
    pub fn error(
        error_type: impl Into<String>,
        error_message: impl Into<String>,
        session_id: impl Into<String>,
        context: SessionContext,
        parent_uuid: Option<String>,
    ) -> Self {
        let error_type_str = error_type.into();
        let error_message_str = error_message.into();

        let mut metadata = HashMap::new();
        metadata.insert(
            "error_type".to_string(),
            Value::String(error_type_str.clone()),
        );

        Self {
            message_type: SessionMessageType::Error,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid,
            branch_id: None,
            branch_parent_uuid: None,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context,
            message: MessageContent {
                role: UnifiedMessageRole::Error,
                content: format!("[{}] {}", error_type_str, error_message_str),
                tool_calls: None,
                tool_results: None,
            },
            usage: None,
            thinking_metadata: None,
            todos: Vec::new(),
            is_sidechain: false,
            metadata,
        }
    }
}

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMessageType {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool result message
    ToolResult,
    /// System message
    System,
    /// Error message
    Error,
    /// Auto-generated summary
    Summary,
    /// Custom title
    CustomTitle,
    /// File history snapshot
    FileHistorySnapshot,
}

impl std::fmt::Display for SessionMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::ToolResult => write!(f, "tool_result"),
            Self::System => write!(f, "system"),
            Self::Error => write!(f, "error"),
            Self::Summary => write!(f, "summary"),
            Self::CustomTitle => write!(f, "custom_title"),
            Self::FileHistorySnapshot => write!(f, "file_history_snapshot"),
        }
    }
}

impl SessionMessageType {
    /// Check if this is a metadata message type (not part of conversation)
    pub fn is_metadata(&self) -> bool {
        matches!(
            self,
            Self::Summary | Self::CustomTitle | Self::FileHistorySnapshot
        )
    }

    /// Check if this is a conversation message type
    pub fn is_conversation(&self) -> bool {
        matches!(
            self,
            Self::User | Self::Assistant | Self::ToolResult | Self::System
        )
    }
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    /// Message role
    pub role: UnifiedMessageRole,

    /// Text content
    pub content: String,

    /// Tool calls (for assistant messages)
    #[serde(rename = "toolCalls")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<UnifiedToolCall>>,

    /// Tool results (for tool_result messages)
    #[serde(rename = "toolResults")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<UnifiedToolResult>>,
}

/// Message role (wire format with camelCase serde)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedMessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool message
    Tool,
    /// Error message
    Error,
}

// ============================================================================
// Tool Types
// ============================================================================

/// Tool call (wire format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolCall {
    /// Tool call ID
    pub id: String,

    /// Tool name
    pub name: String,

    /// Tool arguments
    pub arguments: Value,
}

/// Tool result (wire format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolResult {
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

impl UnifiedToolResult {
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
        }
    }
}

/// Convert from the canonical `ToolResult` to the wire-format `UnifiedToolResult`.
impl From<crate::tools::types::ToolResult> for UnifiedToolResult {
    fn from(result: crate::tools::types::ToolResult) -> Self {
        Self {
            tool_call_id: result.call_id,
            tool_name: result.tool_name,
            content: result.output.unwrap_or_default(),
            success: result.success,
            error: result.error,
        }
    }
}

/// Convert from a reference to the canonical `ToolResult`.
impl From<&crate::tools::types::ToolResult> for UnifiedToolResult {
    fn from(result: &crate::tools::types::ToolResult) -> Self {
        Self {
            tool_call_id: result.call_id.clone(),
            tool_name: result.tool_name.clone(),
            content: result.output.clone().unwrap_or_default(),
            success: result.success,
            error: result.error.clone(),
        }
    }
}

// ============================================================================
// Token Usage
// ============================================================================

/// Token usage statistics (wire format with camelCase serde)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedTokenUsage {
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

    /// Cost estimate (USD)
    #[serde(rename = "costEstimate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_estimate: Option<f64>,
}

impl UnifiedTokenUsage {
    /// Add usage from another UnifiedTokenUsage
    pub fn add(&mut self, other: &UnifiedTokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        if let Some(cost) = other.cost_estimate {
            *self.cost_estimate.get_or_insert(0.0) += cost;
        }
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

// ============================================================================
// File History Snapshot (re-exported from file_tracking module)
// ============================================================================

pub use super::super::file_tracking::{
    FileBackupInfo, FileHistorySnapshot, TrackedFileState, TrackedFilesSnapshot,
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_header_new() {
        let header = SessionHeader::new("test-id", PathBuf::from("/tmp"));
        assert_eq!(header.id, "test-id");
        assert_eq!(header.state, SessionState::Active);
        assert!(!header.is_sidechain);
    }

    #[test]
    fn test_session_message_user() {
        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let msg = SessionMessage::user("Hello", "session-1", ctx);
        assert_eq!(msg.message_type, SessionMessageType::User);
        assert_eq!(msg.message.role, UnifiedMessageRole::User);
        assert_eq!(msg.message.content, "Hello");
    }

    #[test]
    fn test_session_message_chain() {
        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let user_msg = SessionMessage::user("Hello", "session-1", ctx.clone());
        let user_uuid = user_msg.uuid.clone();

        let assistant_msg =
            SessionMessage::assistant("Hi!", "session-1", ctx, Some(user_uuid.clone()));
        assert_eq!(assistant_msg.parent_uuid, Some(user_uuid));
    }

    #[test]
    fn test_token_usage_add() {
        let mut usage1 = UnifiedTokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        };
        let usage2 = UnifiedTokenUsage {
            input_tokens: 200,
            output_tokens: 100,
            ..Default::default()
        };
        usage1.add(&usage2);
        assert_eq!(usage1.input_tokens, 300);
        assert_eq!(usage1.output_tokens, 150);
    }

    #[test]
    fn test_session_record_serialization() {
        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let msg = SessionMessage::user("Test", "session-1", ctx);
        let record = SessionRecord {
            seq: 1,
            timestamp: Utc::now(),
            session_id: "session-1".to_string(),
            payload: SessionRecordPayload::Message(msg),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("recordType"));
        assert!(json.contains("message"));
    }
}
