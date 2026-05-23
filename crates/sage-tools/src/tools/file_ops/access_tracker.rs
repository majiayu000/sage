//! Shared file access tracking for read-before-write enforcement.

use sage_core::tools::base::ToolError;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct FileAccessTracker {
    read_files: RwLock<HashSet<PathBuf>>,
}

impl FileAccessTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn mark_read(&self, canonical_path: PathBuf) {
        self.read_files.write().await.insert(canonical_path);
    }

    pub async fn has_read(&self, canonical_path: &Path) -> bool {
        self.read_files.read().await.contains(canonical_path)
    }

    pub async fn clear(&self) {
        self.read_files.write().await.clear();
    }
}

pub(crate) fn canonicalize_existing_path(
    path: &Path,
    display_path: &str,
) -> Result<PathBuf, ToolError> {
    path.canonicalize().map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to canonicalize '{}': {}. Verify the path exists and is accessible.",
            display_path, e
        ))
    })
}
