//! Grep tool for searching file contents with regex patterns
//!
//! Uses the ripgrep library crates for high-performance searching:
//! - `grep-searcher`: File searching with binary detection
//! - `grep-regex`: Regex pattern matching
//! - `ignore`: Directory walking with .gitignore support

mod filters;
mod output;
mod params;
mod schema;
mod search;

#[cfg(test)]
mod tests;

// Re-export public types for backward compatibility
pub use output::GrepOutputMode;

use sage_core::tools::base::FileSystemTool;
use std::path::PathBuf;

/// Tool for searching files using regex patterns (like ripgrep)
pub struct GrepTool {
    working_directory: PathBuf,
}

impl GrepTool {
    /// Create a new grep tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a grep tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemTool for GrepTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
