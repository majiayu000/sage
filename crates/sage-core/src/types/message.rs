//! Message role type shared across LLM, session, context, and agent modules

use serde::{Deserialize, Serialize};

/// Role of a message in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Error message
    Error,
}

impl Default for MessageRole {
    fn default() -> Self {
        MessageRole::User
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
            MessageRole::Error => write!(f, "error"),
        }
    }
}
