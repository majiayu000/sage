//! Type definitions for the Write tool

use crate::tools::file_ops::access_tracker::{FileAccessTracker, canonicalize_existing_path};
use sage_core::tools::base::ToolError;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Tool for writing files to the filesystem
///
/// This tool can:
/// - Create new files with specified content
/// - Overwrite existing files (with validation)
/// - Create parent directories if they don't exist
///
/// Security features:
/// - Path validation to prevent writing to sensitive locations
/// - Working directory restrictions
/// - Absolute path requirements
pub struct WriteTool {
    pub(crate) working_directory: PathBuf,
    pub(crate) access_tracker: Arc<FileAccessTracker>,
}

impl WriteTool {
    /// Create a new write tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            access_tracker: Arc::new(FileAccessTracker::new()),
        }
    }

    /// Create a write tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
            access_tracker: Arc::new(FileAccessTracker::new()),
        }
    }

    /// Create a write tool with specific working directory and shared access tracker.
    pub fn with_working_directory_and_tracker<P: Into<PathBuf>>(
        working_dir: P,
        access_tracker: Arc<FileAccessTracker>,
    ) -> Self {
        Self {
            working_directory: working_dir.into(),
            access_tracker,
        }
    }

    /// Mark a file as having been read
    ///
    /// This should be called by Read tools to allow subsequent writes
    pub async fn mark_file_as_read(&self, path: PathBuf) -> Result<(), ToolError> {
        let canonical_path = canonicalize_existing_path(&path, &path.display().to_string())?;
        self.access_tracker.mark_read(canonical_path).await;
        Ok(())
    }

    /// Check if a file has been read in this session
    pub(crate) async fn has_been_read(&self, path: &Path) -> Result<bool, ToolError> {
        let canonical_path = canonicalize_existing_path(path, &path.display().to_string())?;
        Ok(self.access_tracker.has_read(&canonical_path).await)
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}
