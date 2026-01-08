//! MultiEdit tool for performing batch edits in a single file
//!
//! **STATUS: DISABLED** - This is a Sage-specific enhanced tool.
//! Kept for potential future use but not registered in the default tool set.
//!
//! This tool follows Claude Code's design pattern for the MultiEdit tool,
//! which allows performing multiple string replacements in a single file
//! in one atomic operation.

mod executor;
mod schema;
mod types;
mod validation;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use std::path::PathBuf;

pub use types::EditOperation;

/// Tool for performing multiple edits in a single file
///
/// This tool allows batch editing of a file by performing multiple
/// string replacements in a single operation. Each edit can optionally
/// replace all occurrences of the target string.
///
/// Key features:
/// - Multiple edits in one call
/// - Optional replace_all for each edit
/// - Atomic operation (all edits succeed or none)
/// - Validates all edits before applying
pub struct MultiEditTool {
    working_directory: PathBuf,
    /// Track files that have been read in this session
    read_files: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<PathBuf>>>,
}

impl MultiEditTool {
    /// Create a new multi-edit tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            read_files: std::sync::Arc::new(
                std::sync::Mutex::new(std::collections::HashSet::new()),
            ),
        }
    }

    /// Create a multi-edit tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
            read_files: std::sync::Arc::new(
                std::sync::Mutex::new(std::collections::HashSet::new()),
            ),
        }
    }

    /// Mark a file as having been read
    pub fn mark_file_as_read(&self, path: PathBuf) {
        if let Ok(mut files) = self.read_files.lock() {
            files.insert(path);
        }
    }
}

impl Default for MultiEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MultiEditTool {
    fn name(&self) -> &str {
        "MultiEdit"
    }

    fn description(&self) -> &str {
        "Performs multiple string replacements in a single file atomically.

Usage:
- You must use the Read tool first to examine the file before editing it.
- All edits are validated before any changes are made.
- If any edit fails validation, no changes are applied.
- Edits are applied in order, so later edits can modify text from earlier edits.

Parameters:
- file_path (required): The absolute path to the file to edit
- edits (required): An array of edit operations, each containing:
  - old_string: The text to replace (must be non-empty)
  - new_string: The replacement text (can be empty to delete)
  - replace_all: Whether to replace all occurrences (default: false)

Notes:
- By default, each old_string must appear exactly once in the file
- Set replace_all=true to replace all occurrences of a string
- Edits are applied sequentially, so later edits see the results of earlier ones
- Only use emojis if the user explicitly requests it"
    }

    fn schema(&self) -> ToolSchema {
        schema::create_schema(self.name(), self.description())
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let edits = validation::parse_edits(call)?;

        let mut result = executor::execute_multi_edit(
            &file_path,
            edits,
            &self.working_directory,
            &self.read_files,
            self.name(),
        )
        .await?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Check required parameters
        if call.get_string("file_path").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'file_path' parameter".to_string(),
            ));
        }

        // Validate edits can be parsed
        validation::parse_edits(call)?;

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(60)) // 1 minute
    }

    fn supports_parallel_execution(&self) -> bool {
        false // File operations should be sequential
    }
}

impl FileSystemTool for MultiEditTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
