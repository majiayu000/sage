//! File tracking and snapshot types for session history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// File history snapshot linked to a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistorySnapshot {
    /// Snapshot type
    #[serde(rename = "type")]
    pub snapshot_type: String,

    /// Associated message ID
    #[serde(rename = "messageId")]
    pub message_id: String,

    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// Whether this is an update to existing snapshot
    #[serde(rename = "isSnapshotUpdate")]
    pub is_snapshot_update: bool,

    /// Actual snapshot data
    pub snapshot: TrackedFilesSnapshot,
}

/// Tracked files snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackedFilesSnapshot {
    /// Tracked files with their state
    #[serde(rename = "trackedFiles")]
    #[serde(default)]
    pub tracked_files: HashMap<String, TrackedFileState>,

    /// File backups for undo
    #[serde(rename = "fileBackups")]
    #[serde(default)]
    pub file_backups: HashMap<String, FileBackupInfo>,
}

/// State of a tracked file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFileState {
    /// Original content (None if file didn't exist)
    #[serde(rename = "originalContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_content: Option<String>,

    /// Content hash
    #[serde(rename = "contentHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    /// File size in bytes
    pub size: u64,

    /// File state (created, modified, deleted, unchanged)
    pub state: String,
}

/// File backup information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBackupInfo {
    /// Path to backup file
    #[serde(rename = "backupPath")]
    pub backup_path: String,

    /// Original content hash
    #[serde(rename = "originalHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hash: Option<String>,
}

impl FileHistorySnapshot {
    /// Create a new file history snapshot
    pub fn new(message_id: impl Into<String>) -> Self {
        Self {
            snapshot_type: "file_history_snapshot".to_string(),
            message_id: message_id.into(),
            timestamp: Utc::now(),
            is_snapshot_update: false,
            snapshot: TrackedFilesSnapshot {
                tracked_files: HashMap::new(),
                file_backups: HashMap::new(),
            },
        }
    }

    /// Create an update snapshot
    pub fn update(message_id: impl Into<String>) -> Self {
        Self {
            snapshot_type: "file_history_snapshot".to_string(),
            message_id: message_id.into(),
            timestamp: Utc::now(),
            is_snapshot_update: true,
            snapshot: TrackedFilesSnapshot {
                tracked_files: HashMap::new(),
                file_backups: HashMap::new(),
            },
        }
    }

    /// Add tracked file
    pub fn with_file(mut self, path: impl Into<String>, state: TrackedFileState) -> Self {
        self.snapshot.tracked_files.insert(path.into(), state);
        self
    }

    /// Add file backup
    pub fn with_backup(mut self, path: impl Into<String>, backup: FileBackupInfo) -> Self {
        self.snapshot.file_backups.insert(path.into(), backup);
        self
    }
}
