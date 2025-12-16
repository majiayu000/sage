//! Checkpoint and versioning system
//!
//! This module provides a checkpoint/versioning system that enables:
//! - State snapshots of files and conversations
//! - Instant rewind to previous states
//! - Pre-tool checkpoints for safe rollback
//! - Incremental change tracking
//!
//! # Overview
//!
//! The checkpoint system is inspired by Claude Code's instant rewind feature,
//! allowing users to restore to any previous state if tool execution causes
//! undesired changes.
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use sage_core::checkpoints::{CheckpointManager, CheckpointManagerConfig, CheckpointType, RestoreOptions};
//!
//! // Create manager
//! let config = CheckpointManagerConfig::new("./project");
//! let manager = CheckpointManager::new(config);
//!
//! // Create a checkpoint before major changes
//! let checkpoint = manager
//!     .create_full_checkpoint("Before refactoring", CheckpointType::Manual)
//!     .await?;
//!
//! // ... make changes ...
//!
//! // Restore if needed
//! let result = manager
//!     .restore(&checkpoint.id, RestoreOptions::files_only())
//!     .await?;
//!
//! println!("Restored {} files", result.restored_count());
//! ```
//!
//! # Checkpoint Types
//!
//! - `Auto` - Automatic checkpoints created by the system
//! - `Manual` - User-requested checkpoints
//! - `PreTool` - Checkpoint before tool execution
//! - `PostSuccess` - Checkpoint after successful operation
//! - `SessionStart` - Checkpoint at session start
//! - `SessionEnd` - Checkpoint at session end
//!
//! # Storage
//!
//! Checkpoints are stored in `.sage/checkpoints/` by default:
//! ```text
//! .sage/checkpoints/
//!   checkpoints/
//!     {checkpoint_id}.json    # Checkpoint metadata
//!   content/
//!     {content_hash}.dat      # Compressed large file content
//! ```

pub mod diff;
pub mod manager;
pub mod storage;
pub mod types;

pub use diff::{ChangeDetector, DiffHunk, DiffLine, FileChange, TextDiff};
pub use manager::{CheckpointManager, CheckpointManagerConfig, RestorePreview};
pub use storage::{CheckpointStorage, CheckpointSummary, FileCheckpointStorage, MemoryCheckpointStorage};
pub use types::{
    Checkpoint, CheckpointId, CheckpointType, ConversationSnapshot, FileSnapshot, FileState,
    RestoreOptions, RestoreResult, TokenUsageSnapshot, ToolExecutionRecord,
};
