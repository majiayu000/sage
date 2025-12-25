//! Core types for JSON editing tool

use sage_core::tools::base::FileSystemTool;
use std::path::PathBuf;

/// Tool for editing JSON files using JSONPath
pub struct JsonEditTool {
    working_directory: PathBuf,
}

impl JsonEditTool {
    /// Create a new JSON edit tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a JSON edit tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }
}

impl Default for JsonEditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemTool for JsonEditTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
