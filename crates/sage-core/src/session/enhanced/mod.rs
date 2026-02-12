//! Enhanced message system with Claude Code-style session tracking
//!
//! The context types (`SessionContext`, `ThinkingMetadata`, etc.) remain
//! defined here as the canonical source. Message types have been moved to
//! `crate::session::types::unified`.

pub mod context;

// Re-export context types (canonical definitions)
pub use context::{SessionContext, ThinkingLevel, ThinkingMetadata, TodoItem, TodoStatus};

// Re-export MessageContent from unified module directly
pub use super::types::unified::MessageContent;
