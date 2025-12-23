//! Edit tool - Claude Code style string replacement based file editing

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;
use tokio::fs;
use tracing::instrument;

/// Edit tool for modifying files using string replacement
/// Matches Claude Code's Edit tool design
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
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        r#"Performs exact string replacements in files.

Usage:
- You must use your `Read` tool at least once in the conversation before editing. This tool will error if you attempt an edit without reading the file.
- When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces) as it appears AFTER the line number prefix. The line number prefix format is: spaces + line number + tab. Everything after that tab is the actual file content to match. Never include any part of the line number prefix in the old_string or new_string.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked.
- The edit will FAIL if `old_string` is not unique in the file. Either provide a larger string with more surrounding context to make it unique or use `replace_all` to change every instance of `old_string`.
- Use `replace_all` for replacing and renaming strings across the file. This parameter is useful if you want to rename a variable for instance."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("file_path", "The absolute path to the file to modify"),
                ToolParameter::string("old_string", "The text to replace"),
                ToolParameter::string(
                    "new_string",
                    "The text to replace it with (must be different from old_string)",
                ),
                ToolParameter::boolean(
                    "replace_all",
                    "Replace all occurrences of old_string (default false)",
                )
                .optional()
                .with_default(serde_json::Value::Bool(false)),
            ],
        )
    }

    #[instrument(skip(self, call), fields(call_id = %call.id))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let old_string = call.get_string("old_string").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'old_string' parameter".to_string())
        })?;

        let new_string = call.get_string("new_string").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'new_string' parameter".to_string())
        })?;

        let replace_all = call.get_bool("replace_all").unwrap_or(false);

        // Validate old_string != new_string
        if old_string == new_string {
            return Err(ToolError::InvalidArguments(
                "No changes to make: old_string and new_string are exactly the same".to_string(),
            ));
        }

        // Validate old_string is not empty
        if old_string.is_empty() {
            return Err(ToolError::InvalidArguments(
                "old_string cannot be empty".to_string(),
            ));
        }

        let path = self.resolve_path(&file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Check if file exists
        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                file_path
            )));
        }

        // Read the file
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read file '{}' for editing: {}. Ensure the file exists and is readable.",
                file_path, e
            ))
        })?;

        // Check if old_string exists
        if !content.contains(&old_string) {
            return Err(ToolError::ExecutionFailed("The string to replace was not found in the file. Make sure the old_string matches exactly, including whitespace and indentation.".to_string()));
        }

        // Count occurrences
        let occurrences = content.matches(&old_string).count();

        // If not replace_all and multiple occurrences, return error
        if !replace_all && occurrences > 1 {
            return Err(ToolError::ExecutionFailed(format!(
                "Found {} occurrences of the string. Use replace_all=true to replace all, or provide more context to make the match unique.",
                occurrences
            )));
        }

        // Perform replacement
        let new_content = if replace_all {
            content.replace(&old_string, &new_string)
        } else {
            content.replacen(&old_string, &new_string, 1)
        };

        // Write back to file
        fs::write(&path, &new_content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to write edited content to '{}' ({} bytes): {}",
                file_path,
                new_content.len(),
                e
            ))
        })?;

        let mut result = if replace_all && occurrences > 1 {
            ToolResult::success(
                &call.id,
                self.name(),
                format!(
                    "Successfully replaced {} occurrences in {}",
                    occurrences, file_path
                ),
            )
        } else {
            ToolResult::success(
                &call.id,
                self.name(),
                format!("Successfully edited {}", file_path),
            )
        };

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        if call.get_string("file_path").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'file_path' parameter".to_string(),
            ));
        }
        if call.get_string("old_string").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'old_string' parameter".to_string(),
            ));
        }
        if call.get_string("new_string").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'new_string' parameter".to_string(),
            ));
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
            "Edit",
            json!({
                "file_path": "test.txt",
                "old_string": "World",
                "new_string": "Rust"
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
    async fn test_edit_tool_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple occurrences
        fs::write(&file_path, "test test test\n").await.unwrap();

        let tool = EditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-2",
            "Edit",
            json!({
                "file_path": "test.txt",
                "old_string": "test",
                "new_string": "replaced",
                "replace_all": true
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify all occurrences were replaced
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "replaced replaced replaced\n");
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
            "Edit",
            json!({
                "file_path": "test.txt",
                "old_string": "NonexistentString",
                "new_string": "replacement"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edit_tool_multiple_occurrences_without_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple occurrences
        fs::write(&file_path, "test test test\n").await.unwrap();

        let tool = EditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "Edit",
            json!({
                "file_path": "test.txt",
                "old_string": "test",
                "new_string": "replaced"
            }),
        );

        // Should fail because multiple occurrences without replace_all
        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edit_tool_same_old_new_string() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = EditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-5",
            "Edit",
            json!({
                "file_path": "test.txt",
                "old_string": "World",
                "new_string": "World"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_tool_schema() {
        let tool = EditTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Edit");
        assert!(!schema.description.is_empty());
    }
}
