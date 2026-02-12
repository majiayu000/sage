//! Session types for persistence and recovery
//!
//! This module defines the core types used in the session system,
//! including session state, messages, and metadata.

// =============================================================================
// Submodules
// =============================================================================

mod base;
mod metadata;
mod session;
pub mod unified;

// =============================================================================
// Re-exports from submodules
// =============================================================================

// Re-export conversation types
pub use super::conversation::{ConversationMessage, SessionToolCall, SessionToolResult};

// Re-export enhanced context types and MessageContent
pub use super::enhanced::{
    MessageContent, SessionContext, ThinkingLevel, ThinkingMetadata, TodoItem, TodoStatus,
};

// Re-export canonical session message types from unified module
pub use unified::{
    SessionMessage, SessionMessageType, WireTokenUsage, UnifiedToolResult,
};

// Re-export file tracking types
pub use super::file_tracking::{
    FileBackupInfo, FileHistorySnapshot, TrackedFileState, TrackedFilesSnapshot,
};

// Re-export base types
pub use base::{MessageRole, SessionId, SessionState, TokenUsage};

// Re-export session types
pub use session::Session;

// Re-export metadata types
pub use metadata::{SessionConfig, SessionSummary};
