//! File reading tool with line numbers and pagination

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;
use tokio::fs;
use tracing::instrument;

/// Maximum line length before truncation
const MAX_LINE_LENGTH: usize = 2000;

/// Default maximum lines to read
const DEFAULT_MAX_LINES: usize = 2000;

/// Tool for reading files with line numbers and pagination
pub struct ReadTool {
    working_directory: PathBuf,
}

impl ReadTool {
    /// Create a new read tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a read tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }

    /// Read file with line numbers
    #[instrument(skip(self), fields(path = %file_path))]
    async fn read_file(
        &self,
        file_path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
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
                "File not found: {}",
                file_path
            )));
        }

        // Check if it's a directory
        if path.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path is a directory, not a file: {}. To list directory contents, use the Bash tool with 'ls -la {}' command.",
                file_path, file_path
            )));
        }

        // Try to detect if it's a binary file by reading first few bytes
        let metadata = fs::metadata(&path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file metadata for '{}': {}", file_path, e)))?;

        // Check if file is too large (> 100MB)
        if metadata.len() > 100 * 1024 * 1024 {
            return Err(ToolError::ExecutionFailed(format!(
                "File too large to read: {} bytes. Use offset and limit parameters for large files.",
                metadata.len()
            )));
        }

        // Try to detect binary files by extension
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                // Image formats
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "svg" => {
                    return Ok(ToolResult::success(
                        "",
                        self.name(),
                        format!(
                            "[Image file detected: {}]\n\nThis is a {} image file. Binary content cannot be displayed as text.\nFile size: {} bytes",
                            file_path,
                            ext_str.to_uppercase(),
                            metadata.len()
                        ),
                    ));
                }
                // PDF format
                "pdf" => {
                    return Ok(ToolResult::success(
                        "",
                        self.name(),
                        format!(
                            "[PDF file detected: {}]\n\nThis is a PDF file. Binary content cannot be displayed as text.\nFile size: {} bytes\n\nTo extract text from PDF, consider using a dedicated PDF processing tool.",
                            file_path,
                            metadata.len()
                        ),
                    ));
                }
                // Other binary formats
                "exe" | "dll" | "so" | "dylib" | "bin" | "zip" | "tar" | "gz" | "rar" | "7z" => {
                    return Ok(ToolResult::success(
                        "",
                        self.name(),
                        format!(
                            "[Binary file detected: {}]\n\nThis is a binary {} file. Content cannot be displayed as text.\nFile size: {} bytes",
                            file_path,
                            ext_str.to_uppercase(),
                            metadata.len()
                        ),
                    ));
                }
                _ => {}
            }
        }

        // Read the file as text
        let content = match fs::read_to_string(&path).await {
            Ok(content) => content,
            Err(e) => {
                // If read_to_string fails, it might be a binary file
                if e.kind() == std::io::ErrorKind::InvalidData {
                    return Ok(ToolResult::success(
                        "",
                        self.name(),
                        format!(
                            "[Binary file detected: {}]\n\nFile contains non-UTF8 data and cannot be displayed as text.\nFile size: {} bytes",
                            file_path,
                            metadata.len()
                        ),
                    ));
                }
                return Err(ToolError::Io(e));
            }
        };

        // Split into lines
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Calculate offset and limit
        let start_line = offset.unwrap_or(0);
        let max_lines = limit.unwrap_or(DEFAULT_MAX_LINES);

        // Handle empty file or offset beyond content
        if total_lines == 0 {
            // Empty file - return success with empty output
            let result = ToolResult::success("", self.name(), "")
                .with_metadata(
                    "file_path",
                    serde_json::Value::String(file_path.to_string()),
                )
                .with_metadata("total_lines", serde_json::Value::Number(0.into()))
                .with_metadata("lines_read", serde_json::Value::Number(0.into()))
                .with_metadata("start_line", serde_json::Value::Number(0.into()))
                .with_metadata("end_line", serde_json::Value::Number(0.into()))
                .with_metadata("truncated", serde_json::Value::Bool(false));

            return Ok(result);
        }

        if start_line >= total_lines {
            return Err(ToolError::InvalidArguments(format!(
                "Offset {} exceeds total lines {} in file",
                start_line, total_lines
            )));
        }

        let end_line = std::cmp::min(start_line + max_lines, total_lines);
        let selected_lines = &lines[start_line..end_line];

        // Format lines with line numbers
        let formatted_lines: Vec<String> = selected_lines
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                let line_num = start_line + idx + 1; // 1-indexed
                let truncated_line = if line.len() > MAX_LINE_LENGTH {
                    format!(
                        "{}... [line truncated, {} chars total]",
                        &line[..MAX_LINE_LENGTH],
                        line.len()
                    )
                } else {
                    line.to_string()
                };
                format!("{:>6}→{}", line_num, truncated_line)
            })
            .collect();

        let output = formatted_lines.join("\n");

        // Build metadata about the read operation
        let truncated = end_line < total_lines;
        let mut result = ToolResult::success("", self.name(), output);

        result = result
            .with_metadata(
                "file_path",
                serde_json::Value::String(file_path.to_string()),
            )
            .with_metadata("total_lines", serde_json::Value::Number(total_lines.into()))
            .with_metadata(
                "lines_read",
                serde_json::Value::Number((end_line - start_line).into()),
            )
            .with_metadata(
                "start_line",
                serde_json::Value::Number((start_line + 1).into()),
            )
            .with_metadata("end_line", serde_json::Value::Number(end_line.into()))
            .with_metadata("truncated", serde_json::Value::Bool(truncated));

        // Add informational message if content was truncated
        if truncated {
            let existing_output = result.output.unwrap_or_default();
            result.output = Some(format!(
                "{}\n\n[Content truncated: showing lines {}-{} of {} total lines. Use offset parameter to read more.]",
                existing_output,
                start_line + 1,
                end_line,
                total_lines
            ));
        }

        Ok(result)
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> &str {
        "Reads a file from the local filesystem with line numbers.

Features:
- Reads files with line numbers in format: '   1→content'
- Supports pagination with offset and limit parameters
- Default limit: 2000 lines
- Truncates lines longer than 2000 characters
- Detects and handles binary files (images, PDFs, executables)
- Provides metadata about the read operation

Usage:
- Read entire file (up to 2000 lines): {\"file_path\": \"/path/to/file.txt\"}
- Read with offset: {\"file_path\": \"/path/to/file.txt\", \"offset\": 100}
- Read with limit: {\"file_path\": \"/path/to/file.txt\", \"limit\": 50}
- Read specific range: {\"file_path\": \"/path/to/file.txt\", \"offset\": 100, \"limit\": 50}

Notes:
- file_path should be an absolute path
- offset is 0-indexed (offset: 0 starts at line 1)
- Line numbers in output are 1-indexed"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("file_path", "Absolute path to the file to read"),
                ToolParameter::number(
                    "offset",
                    "Line number to start reading from (0-indexed, default: 0)",
                )
                .optional(),
                ToolParameter::number("limit", "Maximum number of lines to read (default: 2000)")
                    .optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let offset = call.get_number("offset").map(|n| n as usize);
        let limit = call.get_number("limit").map(|n| n as usize);

        let mut result = self.read_file(&file_path, offset, limit).await?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let _file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        // Validate offset if provided
        if let Some(offset) = call.get_number("offset") {
            if offset < 0.0 {
                return Err(ToolError::InvalidArguments(
                    "Offset must be non-negative".to_string(),
                ));
            }
        }

        // Validate limit if provided
        if let Some(limit) = call.get_number("limit") {
            if limit <= 0.0 {
                return Err(ToolError::InvalidArguments(
                    "Limit must be greater than 0".to_string(),
                ));
            }
            if limit > 10000.0 {
                return Err(ToolError::InvalidArguments(
                    "Limit cannot exceed 10000 lines".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(30) // 30 seconds
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Read operations can run in parallel
    }

    fn is_read_only(&self) -> bool {
        true // This tool only reads data
    }
}

impl FileSystemTool for ReadTool {
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
    async fn test_read_tool_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        let content = "Line 1\nLine 2\nLine 3\n";
        fs::write(&file_path, content).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-1",
            "Read",
            json!({
                "file_path": "test.txt",
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("     1→Line 1"));
        assert!(output.contains("     2→Line 2"));
        assert!(output.contains("     3→Line 3"));
    }

    #[tokio::test]
    async fn test_read_tool_with_offset() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple lines
        let lines: Vec<String> = (1..=10).map(|i| format!("Line {}", i)).collect();
        fs::write(&file_path, lines.join("\n")).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-2",
            "Read",
            json!({
                "file_path": "test.txt",
                "offset": 5,
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("     6→Line 6")); // offset 5 = line 6 (1-indexed)
        assert!(!output.contains("     5→Line 5"));
    }

    #[tokio::test]
    async fn test_read_tool_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple lines
        let lines: Vec<String> = (1..=10).map(|i| format!("Line {}", i)).collect();
        fs::write(&file_path, lines.join("\n")).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-3",
            "Read",
            json!({
                "file_path": "test.txt",
                "limit": 3,
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("     1→Line 1"));
        assert!(output.contains("     2→Line 2"));
        assert!(output.contains("     3→Line 3"));
        assert!(!output.contains("     4→Line 4"));
        assert!(output.contains("truncated")); // Should indicate truncation
    }

    #[tokio::test]
    async fn test_read_tool_with_offset_and_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple lines
        let lines: Vec<String> = (1..=20).map(|i| format!("Line {}", i)).collect();
        fs::write(&file_path, lines.join("\n")).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "Read",
            json!({
                "file_path": "test.txt",
                "offset": 10,
                "limit": 5,
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("    11→Line 11")); // offset 10 = line 11
        assert!(output.contains("    15→Line 15"));
        assert!(!output.contains("    10→Line 10"));
        assert!(!output.contains("    16→Line 16"));
    }

    #[tokio::test]
    async fn test_read_tool_truncate_long_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create a file with a very long line
        let long_line = "a".repeat(3000);
        fs::write(&file_path, &long_line).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-5",
            "Read",
            json!({
                "file_path": "test.txt",
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("line truncated"));
        assert!(output.contains("3000 chars total"));
    }

    #[tokio::test]
    async fn test_read_tool_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ReadTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-6",
            "Read",
            json!({
                "file_path": "nonexistent.txt",
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_read_tool_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-7",
            "Read",
            json!({
                "file_path": "subdir",
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("directory"));
    }

    #[tokio::test]
    async fn test_read_tool_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        let lines: Vec<String> = (1..=5).map(|i| format!("Line {}", i)).collect();
        fs::write(&file_path, lines.join("\n")).await.unwrap();

        let tool = ReadTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-8",
            "Read",
            json!({
                "file_path": "test.txt",
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Check metadata
        assert_eq!(
            result.metadata.get("total_lines").and_then(|v| v.as_u64()),
            Some(5)
        );
        assert_eq!(
            result.metadata.get("lines_read").and_then(|v| v.as_u64()),
            Some(5)
        );
        assert_eq!(
            result.metadata.get("start_line").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            result.metadata.get("end_line").and_then(|v| v.as_u64()),
            Some(5)
        );
        assert_eq!(
            result.metadata.get("truncated").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn test_read_tool_validation_negative_offset() {
        let tool = ReadTool::new();
        let call = create_tool_call(
            "test-9",
            "Read",
            json!({
                "file_path": "test.txt",
                "offset": -1,
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_tool_validation_zero_limit() {
        let tool = ReadTool::new();
        let call = create_tool_call(
            "test-10",
            "Read",
            json!({
                "file_path": "test.txt",
                "limit": 0,
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_tool_validation_excessive_limit() {
        let tool = ReadTool::new();
        let call = create_tool_call(
            "test-11",
            "Read",
            json!({
                "file_path": "test.txt",
                "limit": 20000,
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_tool_schema() {
        let tool = ReadTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Read");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_read_tool_is_read_only() {
        let tool = ReadTool::new();
        assert!(tool.is_read_only());
    }

    #[test]
    fn test_read_tool_supports_parallel() {
        let tool = ReadTool::new();
        assert!(tool.supports_parallel_execution());
    }
}
