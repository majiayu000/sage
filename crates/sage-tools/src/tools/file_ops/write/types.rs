//! Type definitions for the Write tool

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
    /// Track files that have been read in this session
    /// This prevents blind overwrites of files that haven't been examined
    pub(crate) read_files: Arc<Mutex<HashSet<PathBuf>>>,
}

impl WriteTool {
    /// Create a new write tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            read_files: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Create a write tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
            read_files: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Mark a file as having been read
    ///
    /// This should be called by Read tools to allow subsequent writes
    pub fn mark_file_as_read(&self, path: PathBuf) {
        if let Ok(mut files) = self.read_files.lock() {
            files.insert(path);
        }
    }

    /// Check if a file has been read in this session
    pub(crate) fn has_been_read(&self, path: &PathBuf) -> bool {
        if let Ok(files) = self.read_files.lock() {
            files.contains(path)
        } else {
            false
        }
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}
