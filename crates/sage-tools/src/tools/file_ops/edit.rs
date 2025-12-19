//! String replacement based file editing tool

use crate::utils::maybe_truncate;
use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;
use tokio::fs;

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
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Io(e))?;

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
        fs::write(&path, new_content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully replaced '{}' with '{}' in {}",
                old_str, new_str, file_path
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
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::Io(e))?;
        }

        // Write file
        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!("Successfully created file: {}", file_path),
        ))
    }

    /// View file or directory content
    async fn view_file(&self, file_path: &str) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Check if path exists
        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path not found: {}",
                file_path
            )));
        }

        // Handle directory
        if path.is_dir() {
            let mut entries = Vec::new();
            let mut dir_entries = fs::read_dir(&path)
                .await
                .map_err(|e| ToolError::Io(e))?;

            while let Some(entry) = dir_entries.next_entry().await.map_err(|e| ToolError::Io(e))? {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry_path.is_dir();
                entries.push(if is_dir {
                    format!("  {}/", name)
                } else {
                    format!("  {}", name)
                });
            }

            entries.sort();
            let output = format!(
                "Directory: {}\n\n{}\n\n({} items)",
                file_path,
                if entries.is_empty() {
                    "  (empty)".to_string()
                } else {
                    entries.join("\n")
                },
                entries.len()
            );

            return Ok(ToolResult::success("", self.name(), output));
        }

        // Read the file
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Io(e))?;

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
                ToolParameter::string(
                    "command",
                    "The command to execute: 'str_replace', 'create', or 'view'",
                ),
                ToolParameter::string("path", "Path to the file"),
                ToolParameter::optional_string(
                    "old_str",
                    "String to replace (for str_replace command)",
                ),
                ToolParameter::optional_string(
                    "new_str",
                    "Replacement string (for str_replace command)",
                ),
                ToolParameter::optional_string(
                    "file_text",
                    "Content for new file (for create command)",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        let path = call
            .get_string("path")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let mut result = match command.as_str() {
            "str_replace" => {
                let old_str = call.get_string("old_str").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'old_str' parameter for str_replace".to_string(),
                    )
                })?;
                let new_str = call.get_string("new_str").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'new_str' parameter for str_replace".to_string(),
                    )
                })?;
                self.replace_in_file(&path, &old_str, &new_str).await?
            }
            "create" => {
                let content = call.get_string("file_text").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'file_text' parameter for create".to_string(),
                    )
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
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::fs;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_edit_tool_string_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        fs::write(&file_path, "Hello, World!\nThis is a test file.\n")
            .await
            .unwrap();

        let tool = EditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-1",
            "str_replace_based_edit_tool",
            json!({
                "command": "str_replace",
                "path": "test.txt",
                "old_str": "World",
                "new_str": "Rust"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the change
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("Hello, Rust!"));
        assert!(!content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_edit_tool_create_file() {
        let temp_dir = TempDir::new().unwrap();
        let tool = EditTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-2",
            "str_replace_based_edit_tool",
            json!({
                "command": "create",
                "path": "new_file.txt",
                "file_text": "This is a new file content."
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the file was created
        let file_path = temp_dir.path().join("new_file.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("This is a new file content."));
    }

    #[tokio::test]
    async fn test_edit_tool_string_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = EditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-3",
            "str_replace_based_edit_tool",
            json!({
                "command": "str_replace",
                "path": "test.txt",
                "old_str": "NonexistentString",
                "new_str": "replacement"
            }),
        );

        // Implementation returns Err for string not found
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_edit_tool_multiple_occurrences() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple occurrences
        fs::write(&file_path, "test test test\n").await.unwrap();

        let tool = EditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "str_replace_based_edit_tool",
            json!({
                "command": "str_replace",
                "path": "test.txt",
                "old_str": "test",
                "new_str": "replaced"
            }),
        );

        // Implementation returns Err for multiple occurrences (requires more specificity)
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("appears") || err.to_string().contains("times"));
    }

    #[tokio::test]
    async fn test_edit_tool_missing_parameters() {
        let tool = EditTool::new();

        // Missing command - returns Err
        let call = create_tool_call(
            "test-5a",
            "str_replace_based_edit_tool",
            json!({
                "path": "test.txt",
                "old_str": "test",
                "new_str": "replacement"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        // Missing path - returns Err
        let call = create_tool_call(
            "test-5b",
            "str_replace_based_edit_tool",
            json!({
                "command": "str_replace",
                "old_str": "test",
                "new_str": "replacement"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        // Missing old_str for str_replace - returns Err
        let call = create_tool_call(
            "test-5c",
            "str_replace_based_edit_tool",
            json!({
                "command": "str_replace",
                "path": "test.txt",
                "new_str": "replacement"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_tool_schema() {
        let tool = EditTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "str_replace_based_edit_tool");
        assert!(!schema.description.is_empty());
    }
}
