//! Enhanced message system with Claude Code-style session tracking

pub mod context;
pub mod message;

// Re-export all types for convenience
pub use context::{SessionContext, ThinkingLevel, ThinkingMetadata, TodoItem, TodoStatus};
pub use message::{
    EnhancedMessage, EnhancedMessageType, EnhancedTokenUsage, EnhancedToolCall, EnhancedToolResult,
    MessageContent,
};
