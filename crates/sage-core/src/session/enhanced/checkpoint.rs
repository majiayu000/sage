//! Checkpoint data types for session state snapshots and rollback

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Checkpoint data for session state snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    /// Unique checkpoint ID
    pub id: String,

    /// Human-readable description
    pub description: String,

    /// Associated message UUID
    #[serde(rename = "messageUuid")]
    pub message_uuid: String,

    /// File snapshots at this checkpoint
    #[serde(rename = "fileSnapshots")]
    pub file_snapshots: HashMap<String, FileCheckpoint>,

    /// Git state at this checkpoint
    #[serde(rename = "gitState")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_state: Option<GitState>,

    /// Whether this checkpoint can be restored
    #[serde(rename = "canRestore")]
    pub can_restore: bool,
}

impl CheckpointData {
    /// Create a new checkpoint
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        message_uuid: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            message_uuid: message_uuid.into(),
            file_snapshots: HashMap::new(),
            git_state: None,
            can_restore: true,
        }
    }

    /// Add a file snapshot
    pub fn with_file_snapshot(mut self, path: impl Into<String>, snapshot: FileCheckpoint) -> Self {
        self.file_snapshots.insert(path.into(), snapshot);
        self
    }

    /// Set git state
    pub fn with_git_state(mut self, state: GitState) -> Self {
        self.git_state = Some(state);
        self
    }

    /// Mark as non-restorable
    pub fn non_restorable(mut self) -> Self {
        self.can_restore = false;
        self
    }
}

/// File checkpoint data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCheckpoint {
    /// File content hash (SHA-256)
    pub hash: String,

    /// File size in bytes
    pub size: u64,

    /// Path to backup file (if stored)
    #[serde(rename = "backupPath")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

impl FileCheckpoint {
    /// Create a new file checkpoint
    pub fn new(hash: impl Into<String>, size: u64) -> Self {
        Self {
            hash: hash.into(),
            size,
            backup_path: None,
        }
    }

    /// Set backup path
    pub fn with_backup_path(mut self, path: impl Into<String>) -> Self {
        self.backup_path = Some(path.into());
        self
    }
}

/// Git repository state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    /// Current branch name
    pub branch: String,

    /// Current commit hash
    pub commit: String,

    /// Whether there are uncommitted changes
    #[serde(rename = "isDirty")]
    pub is_dirty: bool,

    /// List of staged files
    #[serde(rename = "stagedFiles")]
    #[serde(default)]
    pub staged_files: Vec<String>,

    /// List of modified (unstaged) files
    #[serde(rename = "modifiedFiles")]
    #[serde(default)]
    pub modified_files: Vec<String>,
}

impl GitState {
    /// Create a new git state
    pub fn new(branch: impl Into<String>, commit: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
            commit: commit.into(),
            is_dirty: false,
            staged_files: Vec::new(),
            modified_files: Vec::new(),
        }
    }

    /// Mark as dirty
    pub fn dirty(mut self) -> Self {
        self.is_dirty = true;
        self
    }

    /// Add staged files
    pub fn with_staged_files(mut self, files: Vec<String>) -> Self {
        self.staged_files = files;
        self
    }

    /// Add modified files
    pub fn with_modified_files(mut self, files: Vec<String>) -> Self {
        self.modified_files = files;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_data() {
        let checkpoint = CheckpointData::new("cp-001", "Before refactoring", "msg-123")
            .with_file_snapshot(
                "/src/main.rs",
                FileCheckpoint::new("abc123", 1024).with_backup_path("/backups/main.rs.bak"),
            )
            .with_git_state(
                GitState::new("main", "abc123def")
                    .dirty()
                    .with_modified_files(vec!["src/main.rs".to_string()]),
            );

        assert_eq!(checkpoint.id, "cp-001");
        assert!(checkpoint.can_restore);
        assert!(checkpoint.file_snapshots.contains_key("/src/main.rs"));
        assert!(checkpoint.git_state.is_some());
    }

    #[test]
    fn test_git_state() {
        let state = GitState::new("feature/test", "abc123")
            .dirty()
            .with_staged_files(vec!["file1.rs".to_string()])
            .with_modified_files(vec!["file2.rs".to_string()]);

        assert_eq!(state.branch, "feature/test");
        assert!(state.is_dirty);
        assert_eq!(state.staged_files.len(), 1);
        assert_eq!(state.modified_files.len(), 1);
    }

    #[test]
    fn test_serialization() {
        let checkpoint = CheckpointData::new("cp-001", "Test", "msg-001");
        let json = serde_json::to_string(&checkpoint).unwrap();
        assert!(json.contains("messageUuid"));
        assert!(json.contains("canRestore"));

        let deserialized: CheckpointData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "cp-001");
    }
}
