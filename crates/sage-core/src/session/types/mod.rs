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

// =============================================================================
// Re-exports from submodules
// =============================================================================

// Re-export conversation types
pub use super::conversation::{ConversationMessage, SessionToolCall, SessionToolResult};

// Re-export enhanced message types
pub use super::enhanced::{
    EnhancedMessage, EnhancedMessageType, EnhancedTokenUsage, EnhancedToolCall, EnhancedToolResult,
    MessageContent, SessionContext, ThinkingLevel, ThinkingMetadata, TodoItem, TodoStatus,
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
