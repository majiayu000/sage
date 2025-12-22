//! Enhanced message types for Claude Code-style session tracking

use super::context::{SessionContext, ThinkingMetadata, TodoItem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

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
