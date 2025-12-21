//! File snapshot tracker for undo/redo capability
//!
//! This module tracks file changes during tool execution to enable:
//! - File history snapshots per message
//! - Undo capability to revert file changes
//! - Content backup for safe restoration
//!
//! # Usage
//!
//! ```rust,ignore
//! use sage_core::session::FileSnapshotTracker;
//!
//! let mut tracker = FileSnapshotTracker::new(".sage/backups");
//!
//! // Before modifying files, track them
//! tracker.track_file("src/main.rs").await?;
//!
//! // After tool execution, create snapshot
//! let snapshot = tracker.create_snapshot("msg-123").await?;
//! ```

use crate::error::{SageError, SageResult};
use crate::session::types::{
    FileBackupInfo, FileHistorySnapshot, TrackedFileState, TrackedFilesSnapshot,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

/// File snapshot tracker for capturing file states
pub struct FileSnapshotTracker {
    /// Directory for storing backups
    backup_dir: PathBuf,
    /// Files being tracked: path -> (original_content, original_hash)
    tracked_files: HashMap<PathBuf, TrackedFile>,
    /// Session ID for backup organization
    session_id: Option<String>,
}

/// Internal tracked file state
#[derive(Debug, Clone)]
struct TrackedFile {
    /// Original content (None if file didn't exist)
    original_content: Option<String>,
    /// Original content hash
    original_hash: Option<String>,
    /// Original file size
    original_size: u64,
    /// Backup path
    backup_path: Option<PathBuf>,
}

impl FileSnapshotTracker {
    /// Create a new file snapshot tracker
    pub fn new(backup_dir: impl AsRef<Path>) -> Self {
        Self {
            backup_dir: backup_dir.as_ref().to_path_buf(),
            tracked_files: HashMap::new(),
            session_id: None,
        }
    }

    /// Create a tracker with default backup directory
    pub fn default_tracker() -> Self {
        let backup_dir = std::env::current_dir()
            .unwrap_or_default()
            .join(".sage")
            .join("backups");
        Self::new(backup_dir)
    }

    /// Set the session ID for backup organization
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Track a file before modification
    ///
    /// This captures the current state of the file (if it exists) for potential undo.
    pub async fn track_file(&mut self, path: impl AsRef<Path>) -> SageResult<()> {
        let path = path.as_ref().to_path_buf();

        // Skip if already tracked
        if self.tracked_files.contains_key(&path) {
            debug!("File already tracked: {:?}", path);
            return Ok(());
        }

        let tracked = if path.exists() {
            // Read current content
            let content = fs::read(&path).await.map_err(|e| {
                SageError::storage(format!("Failed to read file for tracking: {}", e))
            })?;

            let size = content.len() as u64;
            let hash = compute_hash(&content);

            // Try to convert to string (for text files)
            let content_str = String::from_utf8(content).ok();

            // Create backup
            let backup_path = self.create_backup(&path, content_str.as_deref()).await?;

            TrackedFile {
                original_content: content_str,
                original_hash: Some(hash),
                original_size: size,
                backup_path,
            }
        } else {
            // File doesn't exist yet - will be created
            TrackedFile {
                original_content: None,
                original_hash: None,
                original_size: 0,
                backup_path: None,
            }
        };

        debug!("Tracking file: {:?}", path);
        self.tracked_files.insert(path, tracked);

        Ok(())
    }

    /// Track multiple files
    pub async fn track_files(&mut self, paths: &[impl AsRef<Path>]) -> SageResult<()> {
        for path in paths {
            self.track_file(path).await?;
        }
        Ok(())
    }

    /// Create a backup of a file
    async fn create_backup(
        &self,
        path: &Path,
        content: Option<&str>,
    ) -> SageResult<Option<PathBuf>> {
        let content = match content {
            Some(c) => c,
            None => return Ok(None),
        };

        // Ensure backup directory exists
        let backup_dir = if let Some(session_id) = &self.session_id {
            self.backup_dir.join(session_id)
        } else {
            self.backup_dir.clone()
        };

        fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| SageError::storage(format!("Failed to create backup directory: {}", e)))?;

        // Generate backup filename
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let timestamp = chrono::Utc::now().timestamp_millis();
        let backup_name = format!("{}_{}.backup", file_name, timestamp);
        let backup_path = backup_dir.join(backup_name);

        // Write backup
        fs::write(&backup_path, content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to write backup file: {}", e)))?;

        debug!("Created backup at: {:?}", backup_path);
        Ok(Some(backup_path))
    }

    /// Create a file history snapshot for the tracked files
    ///
    /// This compares the current state of tracked files to their original state
    /// and generates a snapshot capturing all changes.
    pub async fn create_snapshot(
        &self,
        message_id: impl Into<String>,
    ) -> SageResult<FileHistorySnapshot> {
        let message_id = message_id.into();
        let mut tracked_files = HashMap::new();
        let mut file_backups = HashMap::new();

        for (path, tracked) in &self.tracked_files {
            let path_str = path.to_string_lossy().to_string();

            // Get current state
            let (current_hash, current_size, current_exists) = if path.exists() {
                match fs::read(path).await {
                    Ok(content) => {
                        let hash = compute_hash(&content);
                        (Some(hash), content.len() as u64, true)
                    }
                    Err(e) => {
                        warn!("Failed to read file for snapshot: {:?} - {}", path, e);
                        (None, 0, false)
                    }
                }
            } else {
                (None, 0, false)
            };

            // Determine state
            let state = if tracked.original_content.is_none() && current_exists {
                "created"
            } else if tracked.original_content.is_some() && !current_exists {
                "deleted"
            } else if tracked.original_hash != current_hash {
                "modified"
            } else {
                "unchanged"
            };

            // Add to tracked files
            tracked_files.insert(
                path_str.clone(),
                TrackedFileState {
                    original_content: tracked.original_content.clone(),
                    content_hash: current_hash,
                    size: current_size,
                    state: state.to_string(),
                },
            );

            // Add backup info if available
            if let Some(backup_path) = &tracked.backup_path {
                file_backups.insert(
                    path_str,
                    FileBackupInfo {
                        backup_path: backup_path.to_string_lossy().to_string(),
                        original_hash: tracked.original_hash.clone(),
                    },
                );
            }
        }

        Ok(FileHistorySnapshot {
            snapshot_type: "file_history_snapshot".to_string(),
            message_id,
            timestamp: chrono::Utc::now(),
            is_snapshot_update: false,
            snapshot: TrackedFilesSnapshot {
                tracked_files,
                file_backups,
            },
        })
    }

    /// Restore files to their original state from a snapshot
    pub async fn restore_from_snapshot(
        &self,
        snapshot: &FileHistorySnapshot,
    ) -> SageResult<Vec<String>> {
        let mut restored = Vec::new();

        for (path_str, backup_info) in &snapshot.snapshot.file_backups {
            let path = PathBuf::from(path_str);
            let backup_path = PathBuf::from(&backup_info.backup_path);

            if backup_path.exists() {
                // Read backup content
                let content = fs::read_to_string(&backup_path)
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to read backup: {}", e)))?;

                // Restore file
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        SageError::storage(format!("Failed to create parent directory: {}", e))
                    })?;
                }

                fs::write(&path, content)
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to restore file: {}", e)))?;

                restored.push(path_str.clone());
                debug!("Restored file: {}", path_str);
            } else {
                warn!("Backup file not found: {:?}", backup_path);
            }
        }

        // Handle created files (need to delete them for undo)
        for (path_str, state) in &snapshot.snapshot.tracked_files {
            if state.state == "created" && !snapshot.snapshot.file_backups.contains_key(path_str) {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    if let Err(e) = fs::remove_file(&path).await {
                        warn!(
                            "Failed to remove created file during restore: {} - {}",
                            path_str, e
                        );
                    } else {
                        restored.push(format!("deleted: {}", path_str));
                        debug!("Removed created file: {}", path_str);
                    }
                }
            }
        }

        Ok(restored)
    }

    /// Clear tracked files (call after snapshot is created and saved)
    pub fn clear(&mut self) {
        self.tracked_files.clear();
    }

    /// Get list of tracked file paths
    pub fn tracked_paths(&self) -> Vec<PathBuf> {
        self.tracked_files.keys().cloned().collect()
    }

    /// Check if any files are being tracked
    pub fn is_empty(&self) -> bool {
        self.tracked_files.is_empty()
    }

    /// Get the number of tracked files
    pub fn len(&self) -> usize {
        self.tracked_files.len()
    }
}

/// Compute SHA256 hash of content
fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_track_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut tracker = FileSnapshotTracker::new(temp_dir.path().join("backups"));

        let file_path = temp_dir.path().join("new_file.txt");
        tracker.track_file(&file_path).await.unwrap();

        assert_eq!(tracker.len(), 1);
    }

    #[tokio::test]
    async fn test_track_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.txt");
        fs::write(&file_path, "original content").await.unwrap();

        let mut tracker = FileSnapshotTracker::new(temp_dir.path().join("backups"));
        tracker.track_file(&file_path).await.unwrap();

        assert_eq!(tracker.len(), 1);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "original").await.unwrap();

        let mut tracker = FileSnapshotTracker::new(temp_dir.path().join("backups"));
        tracker.track_file(&file_path).await.unwrap();

        // Modify the file
        fs::write(&file_path, "modified").await.unwrap();

        // Create snapshot
        let snapshot = tracker.create_snapshot("msg-123").await.unwrap();

        assert_eq!(snapshot.message_id, "msg-123");
        assert!(
            snapshot
                .snapshot
                .tracked_files
                .contains_key(&file_path.to_string_lossy().to_string())
        );

        let state = snapshot
            .snapshot
            .tracked_files
            .get(&file_path.to_string_lossy().to_string())
            .unwrap();
        assert_eq!(state.state, "modified");
    }

    #[tokio::test]
    async fn test_restore_from_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "original content").await.unwrap();

        let mut tracker = FileSnapshotTracker::new(temp_dir.path().join("backups"));
        tracker.track_file(&file_path).await.unwrap();

        // Modify the file
        fs::write(&file_path, "modified content").await.unwrap();

        // Create snapshot
        let snapshot = tracker.create_snapshot("msg-123").await.unwrap();

        // Restore
        let restored = tracker.restore_from_snapshot(&snapshot).await.unwrap();
        assert!(!restored.is_empty());

        // Verify restored content
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "original content");
    }

    #[tokio::test]
    async fn test_snapshot_for_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");

        let mut tracker = FileSnapshotTracker::new(temp_dir.path().join("backups"));
        tracker.track_file(&file_path).await.unwrap();

        // Create the file
        fs::write(&file_path, "new content").await.unwrap();

        // Create snapshot
        let snapshot = tracker.create_snapshot("msg-456").await.unwrap();

        let state = snapshot
            .snapshot
            .tracked_files
            .get(&file_path.to_string_lossy().to_string())
            .unwrap();
        assert_eq!(state.state, "created");
    }
}
