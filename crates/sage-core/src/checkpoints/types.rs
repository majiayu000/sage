//! Checkpoint type definitions
//!
//! This module defines the types for the checkpoint/versioning system,
//! enabling state snapshots and instant rewind capabilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unique identifier for a checkpoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(pub String);

impl CheckpointId {
    /// Create a new checkpoint ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from a string
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A checkpoint representing a snapshot of the system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier
    pub id: CheckpointId,

    /// Human-readable name (optional)
    pub name: Option<String>,

    /// Description of what this checkpoint captures
    pub description: String,

    /// When the checkpoint was created
    pub created_at: DateTime<Utc>,

    /// Type of checkpoint
    pub checkpoint_type: CheckpointType,

    /// File snapshots
    pub files: Vec<FileSnapshot>,

    /// Conversation state (optional)
    pub conversation: Option<ConversationSnapshot>,

    /// Tool execution history
    pub tool_history: Vec<ToolExecutionRecord>,

    /// Parent checkpoint ID (for checkpoint chains)
    pub parent_id: Option<CheckpointId>,

    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(description: impl Into<String>, checkpoint_type: CheckpointType) -> Self {
        Self {
            id: CheckpointId::new(),
            name: None,
            description: description.into(),
            created_at: Utc::now(),
            checkpoint_type,
            files: Vec::new(),
            conversation: None,
            tool_history: Vec::new(),
            parent_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set checkpoint name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add file snapshot
    pub fn with_file(mut self, file: FileSnapshot) -> Self {
        self.files.push(file);
        self
    }

    /// Add multiple file snapshots
    pub fn with_files(mut self, files: impl IntoIterator<Item = FileSnapshot>) -> Self {
        self.files.extend(files);
        self
    }

    /// Set conversation snapshot
    pub fn with_conversation(mut self, conversation: ConversationSnapshot) -> Self {
        self.conversation = Some(conversation);
        self
    }

    /// Set parent checkpoint
    pub fn with_parent(mut self, parent_id: CheckpointId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get short ID (first 8 characters)
    pub fn short_id(&self) -> &str {
        &self.id.0[..8.min(self.id.0.len())]
    }

    /// Get file count
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Check if this checkpoint has conversation state
    pub fn has_conversation(&self) -> bool {
        self.conversation.is_some()
    }
}

/// Type of checkpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointType {
    /// Automatic checkpoint before major changes
    Auto,
    /// User-requested checkpoint
    Manual,
    /// Checkpoint before tool execution
    PreTool,
    /// Checkpoint after successful operation
    PostSuccess,
    /// Checkpoint at session start
    SessionStart,
    /// Checkpoint at session end
    SessionEnd,
}

impl std::fmt::Display for CheckpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Manual => write!(f, "manual"),
            Self::PreTool => write!(f, "pre-tool"),
            Self::PostSuccess => write!(f, "post-success"),
            Self::SessionStart => write!(f, "session-start"),
            Self::SessionEnd => write!(f, "session-end"),
        }
    }
}

/// Snapshot of a file's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// File path (relative to project root)
    pub path: PathBuf,

    /// File state
    pub state: FileState,

    /// File permissions (Unix mode)
    pub permissions: Option<u32>,

    /// SHA-256 hash of content (for verification)
    pub content_hash: Option<String>,

    /// File size in bytes
    pub size: u64,
}

impl FileSnapshot {
    /// Create a new file snapshot
    pub fn new(path: impl Into<PathBuf>, state: FileState) -> Self {
        Self {
            path: path.into(),
            state,
            permissions: None,
            content_hash: None,
            size: 0,
        }
    }

    /// Set content hash
    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.content_hash = Some(hash.into());
        self
    }

    /// Set file size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// Set permissions
    pub fn with_permissions(mut self, mode: u32) -> Self {
        self.permissions = Some(mode);
        self
    }
}

/// State of a file in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileState {
    /// File exists with content
    Exists {
        /// Original content (stored externally for large files)
        content: Option<String>,
        /// Reference to external content storage
        content_ref: Option<String>,
    },
    /// File was deleted
    Deleted,
    /// File was created (didn't exist before)
    Created {
        /// Content of the new file
        content: Option<String>,
        content_ref: Option<String>,
    },
    /// File was modified
    Modified {
        /// Original content before modification
        original_content: Option<String>,
        original_content_ref: Option<String>,
        /// New content after modification
        new_content: Option<String>,
        new_content_ref: Option<String>,
    },
}

/// Snapshot of conversation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    /// Session ID
    pub session_id: String,

    /// Message count at snapshot time
    pub message_count: usize,

    /// Last message summary
    pub last_message_summary: Option<String>,

    /// Serialized messages (for restoration)
    pub messages_ref: Option<String>,

    /// Token usage at snapshot
    pub token_usage: TokenUsageSnapshot,
}

/// Token usage at checkpoint time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsageSnapshot {
    /// Total input tokens used
    pub input_tokens: usize,
    /// Total output tokens used
    pub output_tokens: usize,
    /// Total tokens
    pub total_tokens: usize,
}

/// Record of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    /// Tool name
    pub tool_name: String,

    /// Tool call ID
    pub call_id: String,

    /// Arguments passed
    pub arguments: serde_json::Value,

    /// Execution result (success/failure)
    pub success: bool,

    /// Files affected by this tool
    pub affected_files: Vec<PathBuf>,

    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
}

/// Restore options for reverting to a checkpoint
#[derive(Debug, Clone, Default)]
pub struct RestoreOptions {
    /// Restore file states
    pub restore_files: bool,

    /// Restore conversation state
    pub restore_conversation: bool,

    /// Specific files to restore (empty = all)
    pub file_filter: Vec<PathBuf>,

    /// Create a backup checkpoint before restore
    pub create_backup: bool,

    /// Dry run (don't actually restore)
    pub dry_run: bool,
}

impl RestoreOptions {
    /// Create options to restore everything
    pub fn all() -> Self {
        Self {
            restore_files: true,
            restore_conversation: true,
            file_filter: Vec::new(),
            create_backup: true,
            dry_run: false,
        }
    }

    /// Create options to restore only files
    pub fn files_only() -> Self {
        Self {
            restore_files: true,
            restore_conversation: false,
            file_filter: Vec::new(),
            create_backup: true,
            dry_run: false,
        }
    }

    /// Create options to restore only conversation
    pub fn conversation_only() -> Self {
        Self {
            restore_files: false,
            restore_conversation: true,
            file_filter: Vec::new(),
            create_backup: true,
            dry_run: false,
        }
    }

    /// Create dry run options
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Self::all()
        }
    }

    /// Add file filter
    pub fn with_files(mut self, files: impl IntoIterator<Item = PathBuf>) -> Self {
        self.file_filter.extend(files);
        self
    }

    /// Disable backup
    pub fn without_backup(mut self) -> Self {
        self.create_backup = false;
        self
    }
}

/// Result of a restore operation
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Checkpoint that was restored
    pub checkpoint_id: CheckpointId,

    /// Files that were restored
    pub restored_files: Vec<PathBuf>,

    /// Files that failed to restore
    pub failed_files: Vec<(PathBuf, String)>,

    /// Whether conversation was restored
    pub conversation_restored: bool,

    /// Backup checkpoint created (if any)
    pub backup_checkpoint_id: Option<CheckpointId>,

    /// Was this a dry run
    pub was_dry_run: bool,
}

impl RestoreResult {
    /// Check if restore was successful
    pub fn is_success(&self) -> bool {
        self.failed_files.is_empty()
    }

    /// Get count of restored files
    pub fn restored_count(&self) -> usize {
        self.restored_files.len()
    }

    /// Get count of failed files
    pub fn failed_count(&self) -> usize {
        self.failed_files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_id() {
        let id = CheckpointId::new();
        assert!(!id.as_str().is_empty());

        let id2 = CheckpointId::from_string("test-id");
        assert_eq!(id2.as_str(), "test-id");
    }

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint =
            Checkpoint::new("Test checkpoint", CheckpointType::Manual).with_name("My Checkpoint");

        assert_eq!(checkpoint.description, "Test checkpoint");
        assert_eq!(checkpoint.name, Some("My Checkpoint".to_string()));
        assert_eq!(checkpoint.checkpoint_type, CheckpointType::Manual);
    }

    #[test]
    fn test_checkpoint_with_files() {
        let file = FileSnapshot::new(
            "src/main.rs",
            FileState::Exists {
                content: Some("fn main() {}".to_string()),
                content_ref: None,
            },
        );

        let checkpoint = Checkpoint::new("With files", CheckpointType::Auto).with_file(file);

        assert_eq!(checkpoint.file_count(), 1);
    }

    #[test]
    fn test_restore_options() {
        let opts = RestoreOptions::all();
        assert!(opts.restore_files);
        assert!(opts.restore_conversation);
        assert!(opts.create_backup);
        assert!(!opts.dry_run);

        let opts = RestoreOptions::files_only();
        assert!(opts.restore_files);
        assert!(!opts.restore_conversation);
    }

    #[test]
    fn test_checkpoint_type_display() {
        assert_eq!(CheckpointType::Auto.to_string(), "auto");
        assert_eq!(CheckpointType::Manual.to_string(), "manual");
        assert_eq!(CheckpointType::PreTool.to_string(), "pre-tool");
    }
}
