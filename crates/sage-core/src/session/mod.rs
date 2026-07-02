//! Session management system for Sage Agent
//!
//! This module provides session persistence and recovery functionality,
//! allowing conversations to be saved, resumed, and managed across sessions.
//!
//! # Features
//!
//! - JSONL session persistence and recovery
//! - Message history tracking with tool calls and results
//! - Token usage statistics
//! - Session state management (active, paused, completed, failed)
//! - Session caching for persistent state (like Claude Code's ~/.claude.json)
//!
//! # Example
//!
//! ```rust
//! use sage_core::session::{JsonlSessionStorage, SessionContext, SessionMessage};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let temp_dir = tempfile::tempdir()?;
//! let storage = JsonlSessionStorage::new(temp_dir.path());
//! let working_dir = temp_dir.path().to_path_buf();
//! let metadata = storage
//!     .create_session("example-session", working_dir.clone())
//!     .await?;
//! let context = SessionContext::new(working_dir);
//!
//! // Add messages to the session
//! let user_message = SessionMessage::user("Hello!", &metadata.id, context.clone());
//! storage.append_message(&metadata.id, &user_message).await?;
//! let assistant_message =
//!     SessionMessage::assistant("Hi there!", &metadata.id, context, Some(user_message.uuid));
//! storage.append_message(&metadata.id, &assistant_message).await?;
//! # Ok(())
//! # }
//! ```

pub mod branching;
pub mod conversation;
pub mod enhanced;
pub mod file_tracker;
pub mod file_tracking;
pub mod jsonl_storage;
pub mod session_cache;
pub mod summary;
pub mod types;

// Re-export main types
pub use branching::{
    BranchId, BranchManager, BranchNode, BranchSnapshot, SerializedMessage, SerializedToolCall,
    SharedBranchManager, create_branch_manager,
};
pub use file_tracker::FileSnapshotTracker;
pub use jsonl_storage::{JsonlSessionStorage, MessageChainTracker, SessionMetadata};
pub use session_cache::{
    CachedMcpServerConfig, McpServerCache, RecentSession, SessionCache, SessionCacheConfig,
    SessionCacheData, SessionCacheStats, ToolTrustSettings, UserPreferences,
};
pub use summary::SummaryGenerator;
pub use types::{
    ConversationMessage,
    FileBackupInfo,
    FileHistorySnapshot,
    MessageContent,
    MessageRole,
    Session,
    SessionConfig,
    SessionContext,
    // Canonical session message types
    SessionMessage,
    SessionMessageType,
    SessionState,
    SessionSummary,
    SessionToolCall,
    SessionToolResult,
    ThinkingLevel,
    ThinkingMetadata,
    TodoItem,
    TodoStatus,
    TokenUsage,
    TrackedFileState,
    TrackedFilesSnapshot,
    UnifiedToolResult,
    WireTokenUsage,
};
// Re-export UnifiedToolCall as the canonical session-level tool call type
pub use types::unified::UnifiedToolCall;
// Note: SessionId is defined as String type alias in types.rs
// and is re-exported from concurrency module at crate level
