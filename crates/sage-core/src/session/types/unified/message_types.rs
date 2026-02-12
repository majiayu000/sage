//! Message type enum and content struct for session messages.

use serde::{Deserialize, Serialize};

use super::tool_types::{UnifiedToolCall, UnifiedToolResult};
use crate::types::MessageRole;

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMessageType {
    User,
    Assistant,
    ToolResult,
    System,
    Error,
    Summary,
    CustomTitle,
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
    pub role: MessageRole,

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
