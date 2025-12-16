//! MultiEdit tool for performing batch edits in a single file
//!
//! **STATUS: DISABLED** - This is a Sage-specific enhanced tool.
//! Kept for potential future use but not registered in the default tool set.
//!
//! This tool follows Claude Code's design pattern for the MultiEdit tool,
//! which allows performing multiple string replacements in a single file
//! in one atomic operation.

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// A single edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    /// The text to replace
    pub old_string: String,
    /// The replacement text
    pub new_string: String,
    /// Whether to replace all occurrences (default: false)
    #[serde(default)]
    pub replace_all: bool,
}

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

    /// Check if a file has been read in this session
    fn has_been_read(&self, path: &PathBuf) -> bool {
        if let Ok(files) = self.read_files.lock() {
            files.contains(path)
        } else {
            false
        }
    }

    /// Parse edit operations from the tool call
    fn parse_edits(&self, call: &ToolCall) -> Result<Vec<EditOperation>, ToolError> {
        let edits_value = call
            .arguments
            .get("edits")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'edits' parameter".to_string()))?;

        // Try to parse as array of edit operations
        let edits: Vec<EditOperation> = serde_json::from_value(edits_value.clone())
            .map_err(|e| ToolError::InvalidArguments(format!(
                "Invalid 'edits' format: {}. Expected array of {{old_string, new_string, replace_all?}} objects",
                e
            )))?;

        if edits.is_empty() {
            return Err(ToolError::InvalidArguments(
                "The 'edits' array must contain at least one edit operation".to_string(),
            ));
        }

        // Validate each edit
        for (i, edit) in edits.iter().enumerate() {
            if edit.old_string.is_empty() {
                return Err(ToolError::InvalidArguments(format!(
                    "Edit {} has empty 'old_string'. Cannot replace empty strings",
                    i + 1
                )));
            }
            if edit.old_string == edit.new_string {
                return Err(ToolError::InvalidArguments(format!(
                    "Edit {} has identical 'old_string' and 'new_string'. No change would be made",
                    i + 1
                )));
            }
        }

        Ok(edits)
    }

    /// Perform multiple edits on a file
    async fn multi_edit(
        &self,
        file_path: &str,
        edits: Vec<EditOperation>,
    ) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(file_path);

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
                "File does not exist: {}",
                file_path
            )));
        }

        // Check if file has been read (safety check)
        if !self.has_been_read(&path) {
            return Err(ToolError::ValidationFailed(format!(
                "File has not been read: {}. You must use the Read tool first to examine the file before editing it.",
                file_path
            )));
        }

        // Read the file content
        let mut content = fs::read_to_string(&path).await.map_err(ToolError::Io)?;

        // Track edit results
        let mut edit_results = Vec::new();
        let mut total_replacements = 0;

        // Apply each edit in order
        for (i, edit) in edits.iter().enumerate() {
            let occurrences = content.matches(&edit.old_string).count();

            if occurrences == 0 {
                return Err(ToolError::ExecutionFailed(format!(
                    "Edit {}: String '{}' not found in file",
                    i + 1,
                    truncate_for_display(&edit.old_string, 50)
                )));
            }

            if !edit.replace_all && occurrences > 1 {
                return Err(ToolError::ExecutionFailed(format!(
                    "Edit {}: String '{}' appears {} times in file. Either provide more context to make it unique, or set replace_all=true to replace all occurrences",
                    i + 1,
                    truncate_for_display(&edit.old_string, 50),
                    occurrences
                )));
            }

            // Perform the replacement
            if edit.replace_all {
                let new_content = content.replace(&edit.old_string, &edit.new_string);
                content = new_content;
                total_replacements += occurrences;
                edit_results.push(format!(
                    "Edit {}: Replaced {} occurrence(s) of '{}'",
                    i + 1,
                    occurrences,
                    truncate_for_display(&edit.old_string, 30)
                ));
            } else {
                // Replace only the first occurrence
                let new_content = content.replacen(&edit.old_string, &edit.new_string, 1);
                content = new_content;
                total_replacements += 1;
                edit_results.push(format!(
                    "Edit {}: Replaced '{}'",
                    i + 1,
                    truncate_for_display(&edit.old_string, 30)
                ));
            }
        }

        // Write the updated content back to the file
        fs::write(&path, &content).await.map_err(ToolError::Io)?;

        // Format the success message
        let summary = format!(
            "Successfully applied {} edit(s) to {} ({} total replacement(s)):\n{}",
            edits.len(),
            file_path,
            total_replacements,
            edit_results.join("\n")
        );

        Ok(ToolResult::success("", self.name(), summary))
    }
}

/// Truncate a string for display purposes
fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
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
        // Create the items schema for the edits array as serde_json::Value
        let items_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "old_string": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)",
                    "default": false
                }
            },
            "required": ["old_string", "new_string"]
        });

        let mut edits_properties = HashMap::new();
        edits_properties.insert("items".to_string(), items_schema);

        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "file_path",
                    "The absolute path to the file to edit (must be absolute, not relative)",
                ),
                ToolParameter {
                    name: "edits".to_string(),
                    description: "Array of edit operations".to_string(),
                    param_type: "array".to_string(),
                    required: true,
                    default: None,
                    enum_values: None,
                    properties: edits_properties,
                },
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let edits = self.parse_edits(call)?;

        let mut result = self.multi_edit(&file_path, edits).await?;
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
        self.parse_edits(call)?;

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(60) // 1 minute
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

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
    async fn test_multi_edit_single_edit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\nThis is a test file.\n")
            .await
            .unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-1",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "World", "new_string": "Rust"}
                ]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("Hello, Rust!"));
        assert!(!content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_multi_edit_multiple_edits() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\nGoodbye, World!\nTest line.\n")
            .await
            .unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-2",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "Hello, World!", "new_string": "Hello, Rust!"},
                    {"old_string": "Goodbye, World!", "new_string": "Goodbye, Rust!"},
                    {"old_string": "Test line.", "new_string": "Modified line."}
                ]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("3 edit(s)"));

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("Hello, Rust!"));
        assert!(content.contains("Goodbye, Rust!"));
        assert!(content.contains("Modified line."));
    }

    #[tokio::test]
    async fn test_multi_edit_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "foo bar foo baz foo\n")
            .await
            .unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-3",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "foo", "new_string": "qux", "replace_all": true}
                ]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("3 occurrence(s)"));

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "qux bar qux baz qux\n");
    }

    #[tokio::test]
    async fn test_multi_edit_multiple_occurrences_without_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "test test test\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-4",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "test", "new_string": "replaced"}
                ]
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("appears"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_string_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-5",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "nonexistent", "new_string": "replacement"}
                ]
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("not found"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_without_reading() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        // Intentionally NOT marking the file as read

        let call = create_tool_call(
            "test-6",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "World", "new_string": "Rust"}
                ]
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("has not been read"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_empty_old_string() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-7",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "", "new_string": "replacement"}
                ]
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_identical_strings() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-8",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "World", "new_string": "World"}
                ]
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("identical"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_sequential_edits() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        // First edit changes "World" to "Rust", second edit changes "Rust" to "Universe"
        let call = create_tool_call(
            "test-9",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": "World", "new_string": "Rust"},
                    {"old_string": "Rust", "new_string": "Universe"}
                ]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("Hello, Universe!"));
    }

    #[tokio::test]
    async fn test_multi_edit_delete_text() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        // Delete ", World" by replacing with empty string
        let call = create_tool_call(
            "test-10",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": [
                    {"old_string": ", World", "new_string": ""}
                ]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello!\n");
    }

    #[tokio::test]
    async fn test_multi_edit_missing_file_path() {
        let tool = MultiEditTool::new();

        let call = create_tool_call(
            "test-11",
            "MultiEdit",
            json!({
                "edits": [
                    {"old_string": "test", "new_string": "replacement"}
                ]
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("file_path"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_missing_edits() {
        let tool = MultiEditTool::new();

        let call = create_tool_call(
            "test-12",
            "MultiEdit",
            json!({
                "file_path": "test.txt"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("edits"));
        }
    }

    #[tokio::test]
    async fn test_multi_edit_empty_edits_array() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!\n").await.unwrap();

        let tool = MultiEditTool::with_working_directory(temp_dir.path());
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-13",
            "MultiEdit",
            json!({
                "file_path": "test.txt",
                "edits": []
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("at least one"));
        }
    }

    #[test]
    fn test_multi_edit_schema() {
        let tool = MultiEditTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "MultiEdit");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_multi_edit_validation() {
        let tool = MultiEditTool::new();

        // Valid call
        let call = create_tool_call(
            "test-14",
            "MultiEdit",
            json!({
                "file_path": "/path/to/file.txt",
                "edits": [
                    {"old_string": "test", "new_string": "replacement"}
                ]
            }),
        );
        assert!(tool.validate(&call).is_ok());

        // Invalid - missing file_path
        let call = create_tool_call(
            "test-15",
            "MultiEdit",
            json!({
                "edits": [
                    {"old_string": "test", "new_string": "replacement"}
                ]
            }),
        );
        assert!(tool.validate(&call).is_err());

        // Invalid - missing edits
        let call = create_tool_call(
            "test-16",
            "MultiEdit",
            json!({
                "file_path": "/path/to/file.txt"
            }),
        );
        assert!(tool.validate(&call).is_err());
    }

    #[test]
    fn test_truncate_for_display() {
        assert_eq!(truncate_for_display("short", 10), "short");
        assert_eq!(
            truncate_for_display("this is a longer string", 10),
            "this is a ..."
        );
        assert_eq!(truncate_for_display("exact", 5), "exact");
    }
}
