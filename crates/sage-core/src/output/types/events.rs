//! Event types for streaming output

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::base::CostInfo;

/// Output event for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputEvent {
    /// System information
    System(SystemEvent),

    /// Assistant message/response
    Assistant(AssistantEvent),

    /// Tool call started
    ToolCallStart(ToolCallStartEvent),

    /// Tool call result
    ToolCallResult(ToolCallResultEvent),

    /// User prompt processed
    UserPrompt(UserPromptEvent),

    /// Error occurred
    Error(ErrorEvent),

    /// Result/completion
    Result(ResultEvent),
}

impl OutputEvent {
    /// Create a system event
    pub fn system(message: impl Into<String>) -> Self {
        Self::System(SystemEvent {
            message: message.into(),
            timestamp: Utc::now(),
        })
    }

    /// Create an assistant event
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant(AssistantEvent {
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Create a tool call start event
    pub fn tool_start(call_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::ToolCallStart(ToolCallStartEvent {
            call_id: call_id.into(),
            tool_name: name.into(),
            arguments: serde_json::Value::Null,
            timestamp: Utc::now(),
        })
    }

    /// Create a tool call result event
    pub fn tool_result(call_id: impl Into<String>, name: impl Into<String>, success: bool) -> Self {
        Self::ToolCallResult(ToolCallResultEvent {
            call_id: call_id.into(),
            tool_name: name.into(),
            success,
            output: None,
            error: None,
            duration_ms: 0,
            timestamp: Utc::now(),
        })
    }

    /// Create an error event
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(ErrorEvent {
            message: message.into(),
            code: None,
            details: None,
            timestamp: Utc::now(),
        })
    }

    /// Create a result event
    pub fn result(content: impl Into<String>) -> Self {
        Self::Result(ResultEvent {
            content: content.into(),
            cost: None,
            duration_ms: 0,
            session_id: None,
            timestamp: Utc::now(),
        })
    }

    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::System(_) => "system",
            Self::Assistant(_) => "assistant",
            Self::ToolCallStart(_) => "tool_call_start",
            Self::ToolCallResult(_) => "tool_call_result",
            Self::UserPrompt(_) => "user_prompt",
            Self::Error(_) => "error",
            Self::Result(_) => "result",
        }
    }

    /// Get timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::System(e) => e.timestamp,
            Self::Assistant(e) => e.timestamp,
            Self::ToolCallStart(e) => e.timestamp,
            Self::ToolCallResult(e) => e.timestamp,
            Self::UserPrompt(e) => e.timestamp,
            Self::Error(e) => e.timestamp,
            Self::Result(e) => e.timestamp,
        }
    }

    /// Serialize to JSON line
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// System event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Assistant response event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantEvent {
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tool call start event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStartEvent {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl ToolCallStartEvent {
    /// Set arguments
    pub fn with_arguments(mut self, args: serde_json::Value) -> Self {
        self.arguments = args;
        self
    }
}

/// Tool call result event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResultEvent {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl ToolCallResultEvent {
    /// Set output
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Set error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// User prompt event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptEvent {
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl UserPromptEvent {
    /// Create a new user prompt event
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl ErrorEvent {
    /// Set error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set details
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Result/completion event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultEvent {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostInfo>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl ResultEvent {
    /// Set cost info
    pub fn with_cost(mut self, cost: CostInfo) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}
