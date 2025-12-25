//! Directory scanning logic

use crate::error::{SageError, SageResult};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::super::types::FileSnapshot;
use super::capture::ChangeDetector;

impl ChangeDetector {
    /// Scan directory and capture all tracked files
    pub async fn scan_directory(&self, dir: &Path) -> SageResult<Vec<FileSnapshot>> {
        let full_dir = if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            self.base_dir().join(dir)
        };

        let mut snapshots = Vec::new();
        self.scan_recursive(&full_dir, &mut snapshots).await?;
        Ok(snapshots)
    }

    /// Recursive directory scanning
    pub(super) async fn scan_recursive(
        &self,
        dir: &Path,
        snapshots: &mut Vec<FileSnapshot>,
    ) -> SageResult<()> {
        let relative_dir = dir.strip_prefix(self.base_dir()).unwrap_or(dir);

        // Check if directory should be excluded
        if let Some(name) = relative_dir.file_name() {
            if self.is_excluded(&PathBuf::from(name)) {
                return Ok(());
            }
        }

        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            SageError::storage(format!("Failed to read directory {:?}: {}", dir, e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            let metadata = entry
                .metadata()
                .await
                .map_err(|e| SageError::storage(format!("Failed to read metadata: {}", e)))?;

            if metadata.is_dir() {
                Box::pin(self.scan_recursive(&path, snapshots)).await?;
            } else if metadata.is_file() {
                if let Some(snapshot) = self.capture_file(&path).await? {
                    snapshots.push(snapshot);
                }
            }
        }

        Ok(())
    }
}
