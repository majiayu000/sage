//! String replacement based file editing tool

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use crate::utils::maybe_truncate;

/// Tool for editing files using string replacement
pub struct EditTool {
    working_directory: PathBuf,
}

impl EditTool {
    /// Create a new edit tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create an edit tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }

    /// Perform string replacement in a file
    async fn replace_in_file(
        &self,
        file_path: &str,
        old_str: &str,
        new_str: &str,
    ) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Read the file
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::Io(e)
        })?;

        // Check if old_str exists
        if !content.contains(old_str) {
            return Err(ToolError::ExecutionFailed(format!(
                "String '{}' not found in file",
                old_str
            )));
        }

        // Count occurrences
        let occurrences = content.matches(old_str).count();
        if occurrences > 1 {
            return Err(ToolError::ExecutionFailed(format!(
                "String '{}' appears {} times in file. Please be more specific.",
                old_str, occurrences
            )));
        }

        // Perform replacement
        let new_content = content.replace(old_str, new_str);

        // Write back to file
        fs::write(&path, new_content).await.map_err(|e| {
            ToolError::Io(e)
        })?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully replaced '{}' with '{}' in {}",
                old_str,
                new_str,
                file_path
            ),
        ))
    }

    /// Create a new file
    async fn create_file(&self, file_path: &str, content: &str) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Check if file already exists
        if path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File already exists: {}",
                file_path
            )));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::Io(e)
            })?;
        }

        // Write file
        fs::write(&path, content).await.map_err(|e| {
            ToolError::Io(e)
        })?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!("Successfully created file: {}", file_path),
        ))
    }

    /// View file content
    async fn view_file(&self, file_path: &str) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Read the file
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::Io(e)
        })?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!("Content of {}:\n{}", file_path, maybe_truncate(&content)),
        ))
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "str_replace_based_edit_tool"
    }

    fn description(&self) -> &str {
        "Edit files using string replacement. Can create new files, replace text, or view file contents.

Usage patterns:
- View files: Use 'view' action to read file contents (automatically truncated if large)
- View directories: Use 'view' action with directory path to see structure
- Edit files: Use 'str_replace' action with old_str and new_str
- Create files: Use 'create' action with file_path and content

For project exploration: Start with viewing root directory, then README files, then configuration files."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("command", "The command to execute: 'str_replace', 'create', or 'view'"),
                ToolParameter::string("path", "Path to the file"),
                ToolParameter::optional_string("old_str", "String to replace (for str_replace command)"),
                ToolParameter::optional_string("new_str", "Replacement string (for str_replace command)"),
                ToolParameter::optional_string("file_text", "Content for new file (for create command)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call
            .get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        let path = call
            .get_string("path")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let mut result = match command.as_str() {
            "str_replace" => {
                let old_str = call.get_string("old_str").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'old_str' parameter for str_replace".to_string())
                })?;
                let new_str = call.get_string("new_str").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'new_str' parameter for str_replace".to_string())
                })?;
                self.replace_in_file(&path, &old_str, &new_str).await?
            }
            "create" => {
                let content = call.get_string("file_text").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'file_text' parameter for create".to_string())
                })?;
                self.create_file(&path, &content).await?
            }
            "view" => self.view_file(&path).await?,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}. Use 'str_replace', 'create', or 'view'",
                    command
                )));
            }
        };

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let command = call
            .get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        let _path = call
            .get_string("path")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        match command.as_str() {
            "str_replace" => {
                if call.get_string("old_str").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'old_str' parameter for str_replace".to_string(),
                    ));
                }
                if call.get_string("new_str").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'new_str' parameter for str_replace".to_string(),
                    ));
                }
            }
            "create" => {
                if call.get_string("file_text").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'file_text' parameter for create".to_string(),
                    ));
                }
            }
            "view" => {
                // No additional parameters needed
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}. Use 'str_replace', 'create', or 'view'",
                    command
                )));
            }
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(60) // 1 minute
    }

    fn supports_parallel_execution(&self) -> bool {
        false // File operations should be sequential
    }
}

impl FileSystemTool for EditTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
