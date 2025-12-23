//! Conversation types for session management
//!
//! This module contains types related to conversation messages,
//! tool calls, and tool results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::types::MessageRole;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
