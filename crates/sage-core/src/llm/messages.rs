//! LLM message types and structures

use crate::tools::ToolCall;
use crate::types::LLMUsage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of a message in the conversation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions)
    System,
    /// User message (human input)
    User,
    /// Assistant message (AI response)
    Assistant,
    /// Tool message (tool execution result)
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// A message in the LLM conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: String,
    /// Optional tool calls (for assistant messages)
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Optional tool call ID (for tool messages)
    pub tool_call_id: Option<String>,
    /// Optional name (for function/tool messages)
    pub name: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LLMMessage {
    /// Create a new system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message with tool calls
    pub fn assistant_with_tools<S: Into<String>>(content: S, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new tool message
    pub fn tool<S: Into<String>>(content: S, tool_call_id: S, name: Option<S>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: name.map(|n| n.into()),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the message
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if this message has tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls
            .as_ref()
            .map_or(false, |calls| !calls.is_empty())
    }
}

/// Response from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// The response content
    pub content: String,
    /// Tool calls requested by the LLM
    pub tool_calls: Vec<ToolCall>,
    /// Token usage information
    pub usage: Option<LLMUsage>,
    /// Model used for the response
    pub model: Option<String>,
    /// Finish reason
    pub finish_reason: Option<String>,
    /// Response ID from the provider
    pub id: Option<String>,
    /// Additional metadata from the provider
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LLMResponse {
    /// Create a new LLM response
    pub fn new<S: Into<String>>(content: S) -> Self {
        Self {
            content: content.into(),
            tool_calls: Vec::new(),
            usage: None,
            model: None,
            finish_reason: None,
            id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a response with tool calls
    pub fn with_tool_calls<S: Into<String>>(content: S, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content: content.into(),
            tool_calls,
            usage: None,
            model: None,
            finish_reason: None,
            id: None,
            metadata: HashMap::new(),
        }
    }

    /// Add usage information
    pub fn with_usage(mut self, usage: LLMUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Add model information
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Check if the response has tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Check if the response indicates task completion
    pub fn indicates_completion(&self) -> bool {
        self.tool_calls.iter().any(|call| call.name == "task_done")
    }
}
