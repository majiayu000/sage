//! Session entry types for JSONL trajectory storage

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
}

/// A single entry in the session JSONL file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntry {
    /// Session start metadata
    #[serde(rename = "session_start")]
    SessionStart {
        session_id: Uuid,
        task: String,
        provider: String,
        model: String,
        cwd: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
        timestamp: String,
    },

    /// User message
    #[serde(rename = "user")]
    User {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        content: serde_json::Value,
        timestamp: String,
    },

    /// LLM request (recorded before sending)
    #[serde(rename = "llm_request")]
    LlmRequest {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        messages: Vec<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools: Option<Vec<String>>,
        timestamp: String,
    },

    /// LLM response
    #[serde(rename = "llm_response")]
    LlmResponse {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        content: String,
        model: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<serde_json::Value>>,
        timestamp: String,
    },

    /// Tool call (recorded before execution)
    #[serde(rename = "tool_call")]
    ToolCall {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        tool_name: String,
        tool_input: serde_json::Value,
        timestamp: String,
    },

    /// Tool result
    #[serde(rename = "tool_result")]
    ToolResult {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        tool_name: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        execution_time_ms: u64,
        timestamp: String,
    },

    /// Error occurred
    #[serde(rename = "error")]
    Error {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        error_type: String,
        message: String,
        timestamp: String,
    },

    /// Session completion
    #[serde(rename = "session_end")]
    SessionEnd {
        uuid: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<Uuid>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        final_result: Option<String>,
        total_steps: u32,
        execution_time_secs: f64,
        timestamp: String,
    },
}

impl SessionEntry {
    /// Get the UUID of this entry
    pub fn uuid(&self) -> Uuid {
        match self {
            Self::SessionStart { session_id, .. } => *session_id,
            Self::User { uuid, .. } => *uuid,
            Self::LlmRequest { uuid, .. } => *uuid,
            Self::LlmResponse { uuid, .. } => *uuid,
            Self::ToolCall { uuid, .. } => *uuid,
            Self::ToolResult { uuid, .. } => *uuid,
            Self::Error { uuid, .. } => *uuid,
            Self::SessionEnd { uuid, .. } => *uuid,
        }
    }

    /// Get the parent UUID of this entry
    pub fn parent_uuid(&self) -> Option<Uuid> {
        match self {
            Self::SessionStart { .. } => None,
            Self::User { parent_uuid, .. } => *parent_uuid,
            Self::LlmRequest { parent_uuid, .. } => *parent_uuid,
            Self::LlmResponse { parent_uuid, .. } => *parent_uuid,
            Self::ToolCall { parent_uuid, .. } => *parent_uuid,
            Self::ToolResult { parent_uuid, .. } => *parent_uuid,
            Self::Error { parent_uuid, .. } => *parent_uuid,
            Self::SessionEnd { parent_uuid, .. } => *parent_uuid,
        }
    }

    /// Get the timestamp of this entry
    pub fn timestamp(&self) -> &str {
        match self {
            Self::SessionStart { timestamp, .. } => timestamp,
            Self::User { timestamp, .. } => timestamp,
            Self::LlmRequest { timestamp, .. } => timestamp,
            Self::LlmResponse { timestamp, .. } => timestamp,
            Self::ToolCall { timestamp, .. } => timestamp,
            Self::ToolResult { timestamp, .. } => timestamp,
            Self::Error { timestamp, .. } => timestamp,
            Self::SessionEnd { timestamp, .. } => timestamp,
        }
    }

    /// Get the entry type as string
    pub fn entry_type(&self) -> &'static str {
        match self {
            Self::SessionStart { .. } => "session_start",
            Self::User { .. } => "user",
            Self::LlmRequest { .. } => "llm_request",
            Self::LlmResponse { .. } => "llm_response",
            Self::ToolCall { .. } => "tool_call",
            Self::ToolResult { .. } => "tool_result",
            Self::Error { .. } => "error",
            Self::SessionEnd { .. } => "session_end",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_entry_serialization() {
        let entry = SessionEntry::SessionStart {
            session_id: Uuid::new_v4(),
            task: "Test task".to_string(),
            provider: "glm".to_string(),
            model: "glm-4.7".to_string(),
            cwd: "/test".to_string(),
            git_branch: Some("main".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"type\":\"session_start\""));

        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entry_type(), "session_start");
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: Some(10),
            cache_write_tokens: None,
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"input_tokens\":100"));
        assert!(!json.contains("cache_write_tokens")); // skip_serializing_if
    }
}
