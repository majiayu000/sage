//! Type definitions for the glob tool

use std::path::PathBuf;

/// Maximum number of files to return
pub const MAX_FILES: usize = 1000;

/// Tool for finding files using glob patterns
pub struct GlobTool {
    pub(crate) working_directory: PathBuf,
}

impl GlobTool {
    /// Create a new glob tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a glob tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}
