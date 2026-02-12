//! Unified session message model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::header::{BranchId, MessageId, SessionId};
use super::message_types::{MessageContent, SessionMessageType};
use super::tool_types::{UnifiedToolCall, UnifiedToolResult};
use super::wire_token::WireTokenUsage;
use crate::session::enhanced::context::{SessionContext, ThinkingMetadata, TodoItem};
use crate::types::MessageRole;

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
    pub usage: Option<WireTokenUsage>,

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
                role: MessageRole::User,
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
                role: MessageRole::Assistant,
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
                role: MessageRole::Tool,
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
    pub fn with_usage(mut self, usage: WireTokenUsage) -> Self {
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
                role: MessageRole::Error,
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
