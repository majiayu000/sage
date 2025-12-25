//! Core write logic and file operations

use super::types::WriteTool;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::ToolResult;
use tokio::fs;

impl WriteTool {
    /// Write content to a file
    pub(crate) async fn write_file(
        &self,
        file_path: &str,
        content: &str,
    ) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Check if file exists and hasn't been read
        let file_exists = path.exists();
        if file_exists && !self.has_been_read(&path) {
            return Err(ToolError::ValidationFailed(format!(
                "File exists but has not been read: {}. You must use the Read tool first to examine the file before overwriting it.",
                file_path
            )));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionFailed(format!(
                    "Failed to create parent directories for '{}': {}",
                    file_path, e
                ))
            })?;
        }

        // Write the file
        fs::write(&path, content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to write content to file '{}': {}",
                file_path, e
            ))
        })?;

        // Mark as read for future operations
        self.mark_file_as_read(path.clone());

        let action = if file_exists {
            "overwritten"
        } else {
            "created"
        };
        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully {} file: {} ({} bytes)",
                action,
                file_path,
                content.len()
            ),
        ))
    }
}
