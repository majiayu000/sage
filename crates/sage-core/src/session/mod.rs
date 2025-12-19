//! Session management system for Sage Agent
//!
//! This module provides session persistence and recovery functionality,
//! allowing conversations to be saved, resumed, and managed across sessions.
//!
//! # Features
//!
//! - Session creation with configurable working directory and model
//! - Message history tracking with tool calls and results
//! - Token usage statistics
//! - Session state management (active, paused, completed, failed)
//! - File-based and in-memory storage backends
//! - Session caching for persistent state (like Claude Code's ~/.claude.json)
//!
//! # Example
//!
//! ```rust
//! use sage_core::session::{SessionManager, SessionConfig, ConversationMessage};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a session manager with in-memory storage
//! let manager = SessionManager::in_memory();
//!
//! // Create a new session
//! let config = SessionConfig::new()
//!     .with_name("My Session")
//!     .with_model("claude-3");
//!
//! let session_id = manager.create(config).await?;
//!
//! // Add messages to the session
//! manager.add_message(&session_id, ConversationMessage::user("Hello!")).await?;
//! manager.add_message(&session_id, ConversationMessage::assistant("Hi there!")).await?;
//!
//! // Get session info
//! let session = manager.get(&session_id).await?.unwrap();
//! println!("Session has {} messages", session.message_count());
//!
//! // Complete the session
//! manager.complete(&session_id).await?;
//! # Ok(())
//! # }
//! ```

pub mod branching;
pub mod manager;
pub mod session_cache;
pub mod storage;
pub mod types;

// Re-export main types
pub use branching::{
    BranchId, BranchManager, BranchNode, BranchSnapshot, SerializedMessage, SerializedToolCall,
    SharedBranchManager, create_branch_manager,
};
pub use manager::SessionManager;
pub use session_cache::{
    MCPServerCache, MCPServerConfig, RecentSession, SessionCache, SessionCacheConfig,
    SessionCacheData, SessionCacheStats, ToolTrustSettings, UserPreferences,
};
pub use storage::{BoxedSessionStorage, FileSessionStorage, MemorySessionStorage, SessionStorage};
pub use types::{
    ConversationMessage, MessageRole, Session, SessionConfig, SessionState,
    SessionSummary, SessionToolCall, SessionToolResult, TokenUsage,
};
// Note: SessionId is defined as String type alias in types.rs
// and is re-exported from concurrency module at crate level
