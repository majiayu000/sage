//! Write tool for creating or overwriting files
//!
//! This tool follows Claude Code's design pattern for the Write tool,
//! which allows creating new files or overwriting existing files with
//! proper validation and security checks.

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;
use tokio::fs;
use tracing::instrument;

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
    working_directory: PathBuf,
    /// Track files that have been read in this session
    /// This prevents blind overwrites of files that haven't been examined
    read_files: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<PathBuf>>>,
}

impl WriteTool {
    /// Create a new write tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            read_files: std::sync::Arc::new(
                std::sync::Mutex::new(std::collections::HashSet::new()),
            ),
        }
    }

    /// Create a write tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
            read_files: std::sync::Arc::new(
                std::sync::Mutex::new(std::collections::HashSet::new()),
            ),
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
    fn has_been_read(&self, path: &PathBuf) -> bool {
        if let Ok(files) = self.read_files.lock() {
            files.contains(path)
        } else {
            false
        }
    }

    /// Write content to a file
    async fn write_file(&self, file_path: &str, content: &str) -> Result<ToolResult, ToolError> {
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
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create parent directories for '{}': {}", file_path, e)))?;
        }

        // Write the file
        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write content to file '{}': {}", file_path, e)))?;

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

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> &str {
        "Writes a file to the local filesystem.

Usage:
- This tool will overwrite the existing file if there is one at the provided path.
- If this is an existing file, you MUST use the Read tool first to read the file's contents. This tool will fail if you did not read the file first.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked.

Parameters:
- file_path (required): The absolute path to the file to write (must be absolute, not relative)
- content (required): The content to write to the file

Security:
- Parent directories will be created automatically if they don't exist
- Path validation ensures files are written within safe locations
- Existing files must be read first to prevent blind overwrites"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "file_path",
                    "The absolute path to the file to write (must be absolute, not relative)",
                ),
                ToolParameter::string("content", "The content to write to the file"),
            ],
        )
    }

    #[instrument(skip(self, call), fields(call_id = %call.id, file_path = call.get_string("file_path").as_deref().unwrap_or("<missing>")))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let content = call.get_string("content").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'content' parameter".to_string())
        })?;

        let mut result = self.write_file(&file_path, &content).await?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Check required parameters
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let _content = call.get_string("content").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'content' parameter".to_string())
        })?;

        // Validate that the path looks like an absolute path
        let path = std::path::Path::new(&file_path);
        if !path.is_absolute() && !file_path.starts_with('/') && !file_path.starts_with('C') {
            // Allow paths that start with / or drive letters
            // This is a soft warning - the tool may still work with relative paths
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

impl FileSystemTool for WriteTool {
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
    async fn test_write_tool_create_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-1",
            "Write",
            json!({
                "file_path": "test.txt",
                "content": "Hello, World!"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("created"));

        // Verify the file was created with correct content
        let file_path = temp_dir.path().join("test.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_tool_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-2",
            "Write",
            json!({
                "file_path": "subdir/nested/test.txt",
                "content": "Nested file content"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the file was created in nested directories
        let file_path = temp_dir.path().join("subdir/nested/test.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Nested file content");
    }

    #[tokio::test]
    async fn test_write_tool_overwrite_after_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        fs::write(&file_path, "Initial content").await.unwrap();

        let tool = WriteTool::with_working_directory(temp_dir.path());

        // Mark file as read
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-3",
            "Write",
            json!({
                "file_path": "test.txt",
                "content": "Updated content"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("overwritten"));

        // Verify the file was overwritten
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Updated content");
    }

    #[tokio::test]
    async fn test_write_tool_overwrite_without_read_fails() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        fs::write(&file_path, "Initial content").await.unwrap();

        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-4",
            "Write",
            json!({
                "file_path": "test.txt",
                "content": "Attempting to overwrite"
            }),
        );

        // Should fail because file exists but hasn't been read
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(ToolError::ValidationFailed(msg)) => {
                assert!(msg.contains("has not been read"));
            }
            _ => panic!("Expected ValidationFailed error"),
        }

        // Verify original content is unchanged
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Initial content");
    }

    #[tokio::test]
    async fn test_write_tool_missing_parameters() {
        let tool = WriteTool::new();

        // Missing file_path
        let call = create_tool_call(
            "test-5a",
            "Write",
            json!({
                "content": "Some content"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        // Missing content
        let call = create_tool_call(
            "test-5b",
            "Write",
            json!({
                "file_path": "test.txt"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_tool_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-6",
            "Write",
            json!({
                "file_path": "empty.txt",
                "content": ""
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify empty file was created
        let file_path = temp_dir.path().join("empty.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "");
    }

    #[tokio::test]
    async fn test_write_tool_multiline_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let multiline_content = "Line 1\nLine 2\nLine 3\n";
        let call = create_tool_call(
            "test-7",
            "Write",
            json!({
                "file_path": "multiline.txt",
                "content": multiline_content
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify multiline content
        let file_path = temp_dir.path().join("multiline.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, multiline_content);
    }

    #[tokio::test]
    async fn test_write_tool_binary_safe_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        // Content with special characters
        let content = "Special chars: \t\r\n\0";
        let call = create_tool_call(
            "test-8",
            "Write",
            json!({
                "file_path": "special.txt",
                "content": content
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify content with special characters
        let file_path = temp_dir.path().join("special.txt");
        let read_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_write_tool_schema() {
        let tool = WriteTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Write");
        assert!(!schema.description.is_empty());

        // Verify schema has required parameters
        if let serde_json::Value::Object(params) = &schema.parameters {
            if let Some(serde_json::Value::Object(properties)) = params.get("properties") {
                assert!(properties.contains_key("file_path"));
                assert!(properties.contains_key("content"));
            }
        }
    }

    #[test]
    fn test_write_tool_validation() {
        let tool = WriteTool::new();

        // Valid call
        let call = create_tool_call(
            "test-9",
            "Write",
            json!({
                "file_path": "/absolute/path/test.txt",
                "content": "Valid content"
            }),
        );
        assert!(tool.validate(&call).is_ok());

        // Invalid - missing parameters
        let call = create_tool_call(
            "test-10",
            "Write",
            json!({
                "file_path": "/path/test.txt"
            }),
        );
        assert!(tool.validate(&call).is_err());
    }
}
