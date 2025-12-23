//! File rotation utilities for trajectory storage

use crate::error::SageError;
use crate::error::SageResult;
use std::path::PathBuf;
use tokio::fs;

use super::file_storage::FileStorage;

/// Information about a trajectory file for rotation purposes
#[derive(Debug)]
pub(super) struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: std::time::SystemTime,
}

impl FileStorage {
    /// Perform file rotation based on configured limits
    ///
    /// This method enforces trajectory storage limits by:
    /// 1. Deleting oldest files when max_trajectories is exceeded
    /// 2. Deleting oldest files when total_size_limit is exceeded
    ///
    /// Files are sorted by modification time, with oldest files deleted first.
    ///
    /// # Example
    /// ```no_run
    /// use sage_core::trajectory::storage::{FileStorage, RotationConfig};
    /// # use sage_core::error::SageResult;
    /// # async fn example() -> SageResult<()> {
    /// let rotation = RotationConfig::with_max_trajectories(10);
    /// let storage = FileStorage::with_config("trajectories", true, rotation)?;
    /// // Rotation happens automatically after save
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rotate_files(&self) -> SageResult<()> {
        // Only perform rotation if we're using directory mode
        if !self.is_directory_path() {
            return Ok(());
        }

        // If no rotation limits are set, nothing to do
        if self.rotation_config().max_trajectories.is_none()
            && self.rotation_config().total_size_limit.is_none()
        {
            return Ok(());
        }

        if !self.base_path().exists() {
            return Ok(());
        }

        // Collect all trajectory files with metadata
        let mut files = self.collect_file_info().await?;

        // Sort by modification time (oldest first)
        files.sort_by_key(|f| f.modified);

        // Apply max_trajectories limit
        self.apply_count_limit(&mut files).await?;

        // Apply total_size_limit
        self.apply_size_limit(&mut files).await?;

        Ok(())
    }

    /// Collect information about all trajectory files
    async fn collect_file_info(&self) -> SageResult<Vec<FileInfo>> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(self.base_path()).await.map_err(|e| {
            SageError::config(format!(
                "Failed to read trajectory directory {:?}: {}",
                self.base_path(), e
            ))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::config(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            let extension = path.extension().and_then(|s| s.to_str());

            // Only consider .json and .gz files
            if extension == Some("json") || extension == Some("gz") {
                if let Ok(metadata) = fs::metadata(&path).await {
                    if let Ok(modified) = metadata.modified() {
                        files.push(FileInfo {
                            path: path.clone(),
                            size: metadata.len(),
                            modified,
                        });
                    }
                }
            }
        }

        Ok(files)
    }

    /// Apply max trajectories count limit
    async fn apply_count_limit(&self, files: &mut Vec<FileInfo>) -> SageResult<()> {
        if let Some(max_trajectories) = self.rotation_config().max_trajectories {
            while files.len() > max_trajectories {
                if let Some(oldest) = files.first() {
                    tracing::info!(
                        "Rotating trajectory file (max count): {}",
                        oldest.path.display()
                    );
                    fs::remove_file(&oldest.path).await.map_err(|e| {
                        SageError::io(format!(
                            "Failed to delete trajectory file {:?}: {}",
                            oldest.path, e
                        ))
                    })?;
                    files.remove(0);
                }
            }
        }
        Ok(())
    }

    /// Apply total size limit
    async fn apply_size_limit(&self, files: &mut Vec<FileInfo>) -> SageResult<()> {
        if let Some(size_limit) = self.rotation_config().total_size_limit {
            let mut total_size: u64 = files.iter().map(|f| f.size).sum();

            while total_size > size_limit && !files.is_empty() {
                if let Some(oldest) = files.first() {
                    tracing::info!(
                        "Rotating trajectory file (size limit): {} ({} bytes)",
                        oldest.path.display(),
                        oldest.size
                    );
                    total_size = total_size.saturating_sub(oldest.size);
                    fs::remove_file(&oldest.path).await.map_err(|e| {
                        SageError::io(format!(
                            "Failed to delete trajectory file {:?}: {}",
                            oldest.path, e
                        ))
                    })?;
                    files.remove(0);
                }
            }
        }
        Ok(())
    }
}
