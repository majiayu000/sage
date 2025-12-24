//! Checkpoint restore operations
//!
//! This module handles file restoration from checkpoints.

use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::error::{SageError, SageResult};
use super::types::{FileSnapshot, FileState};

/// Preview of what will happen during restore
#[derive(Debug, Clone)]
pub enum RestorePreview {
    /// File will be created
    WillCreate(PathBuf),
    /// File will be overwritten
    WillOverwrite(PathBuf),
    /// File will be reverted to original
    WillRevert(PathBuf),
    /// File will be deleted
    WillDelete(PathBuf),
    /// No change needed
    NoChange(PathBuf),
}

impl RestorePreview {
    /// Get the path
    pub fn path(&self) -> &Path {
        match self {
            Self::WillCreate(p)
            | Self::WillOverwrite(p)
            | Self::WillRevert(p)
            | Self::WillDelete(p)
            | Self::NoChange(p) => p,
        }
    }
}

/// Restore a single file from a snapshot
pub async fn restore_file(project_root: &Path, snapshot: &FileSnapshot) -> SageResult<()> {
    let full_path = project_root.join(&snapshot.path);

    match &snapshot.state {
        FileState::Exists { content, .. } | FileState::Created { content, .. } => {
            if let Some(content) = content {
                // Ensure parent directory exists
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        SageError::storage(format!("Failed to create directory: {}", e))
                    })?;
                }

                // Write content
                let mut file = fs::File::create(&full_path)
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to create file: {}", e)))?;
                file.write_all(content.as_bytes())
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to write file: {}", e)))?;

                // Restore permissions
                #[cfg(unix)]
                if let Some(mode) = snapshot.permissions {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(mode);
                    fs::set_permissions(&full_path, perms).await.map_err(|e| {
                        SageError::storage(format!("Failed to set permissions: {}", e))
                    })?;
                }
            }
        }
        FileState::Modified {
            original_content, ..
        } => {
            // Restore to original content
            if let Some(content) = original_content {
                let mut file = fs::File::create(&full_path)
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to create file: {}", e)))?;
                file.write_all(content.as_bytes())
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to write file: {}", e)))?;
            }
        }
        FileState::Deleted => {
            // File was deleted in this snapshot, remove it if it exists
            if full_path.exists() {
                fs::remove_file(&full_path)
                    .await
                    .map_err(|e| SageError::storage(format!("Failed to delete file: {}", e)))?;
            }
        }
    }

    tracing::debug!("Restored file: {:?}", snapshot.path);
    Ok(())
}

/// Preview what will happen when restoring a file
pub async fn preview_file_restore(
    project_root: &Path,
    snapshot: &FileSnapshot,
) -> SageResult<RestorePreview> {
    let full_path = project_root.join(&snapshot.path);
    let exists = full_path.exists();

    let preview = match &snapshot.state {
        FileState::Exists { .. } | FileState::Created { .. } => {
            if exists {
                RestorePreview::WillOverwrite(snapshot.path.clone())
            } else {
                RestorePreview::WillCreate(snapshot.path.clone())
            }
        }
        FileState::Modified { .. } => RestorePreview::WillRevert(snapshot.path.clone()),
        FileState::Deleted => {
            if exists {
                RestorePreview::WillDelete(snapshot.path.clone())
            } else {
                RestorePreview::NoChange(snapshot.path.clone())
            }
        }
    };

    Ok(preview)
}
