//! Core file reading logic with line numbers and pagination

use super::binary::{create_binary_result, handle_binary_file};
use super::types::{DEFAULT_MAX_LINES, MAX_FILE_SIZE, MAX_LINE_LENGTH};
use sage_core::tools::base::{FileSystemTool, ToolError};
use sage_core::tools::types::ToolResult;
use tokio::fs;

/// Read a file with line numbers and pagination
pub async fn read_file<T: FileSystemTool>(
    tool: &T,
    tool_name: &str,
    file_path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<ToolResult, ToolError> {
    let path = tool.resolve_path(file_path);

    // Security check
    if !tool.is_safe_path(&path) {
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

    // Get file metadata
    let metadata = fs::metadata(&path).await.map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to read file metadata for '{}': {}. Ensure the file exists and you have permission to access it.",
            file_path, e
        ))
    })?;

    // Check if file is too large (> 100MB)
    if metadata.len() > MAX_FILE_SIZE as u64 {
        return Err(ToolError::ExecutionFailed(format!(
            "File too large to read: {} bytes. Use offset and limit parameters for large files.",
            metadata.len()
        )));
    }

    // Try to detect binary files by extension
    if let Some(binary_result) = handle_binary_file(&path, file_path, tool_name).await? {
        return Ok(binary_result);
    }

    // Read the file as text
    let content = match fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(e) => {
            // If read_to_string fails, it might be a binary file
            if e.kind() == std::io::ErrorKind::InvalidData {
                return Ok(create_binary_result(file_path, metadata.len(), tool_name));
            }
            return Err(ToolError::Io(e));
        }
    };

    format_content(tool_name, file_path, &content, offset, limit)
}

/// Format file content with line numbers and pagination
fn format_content(
    tool_name: &str,
    file_path: &str,
    content: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<ToolResult, ToolError> {
    // Split into lines
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Calculate offset and limit
    let start_line = offset.unwrap_or(0);
    let max_lines = limit.unwrap_or(DEFAULT_MAX_LINES);

    // Handle empty file or offset beyond content
    if total_lines == 0 {
        return Ok(create_empty_result(tool_name, file_path));
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
                // Truncate on a valid UTF-8 boundary to avoid panics when
                // MAX_LINE_LENGTH falls in the middle of a multi-byte char.
                let safe_end = line
                    .char_indices()
                    .take_while(|(i, _)| *i < MAX_LINE_LENGTH)
                    .map(|(i, ch)| i + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                let safe_prefix = &line[..safe_end];
                format!(
                    "{}... [line truncated, {} chars total]",
                    safe_prefix,
                    line.len()
                )
            } else {
                line.to_string()
            };
            format!("{:>6}→{}", line_num, truncated_line)
        })
        .collect();

    let output = formatted_lines.join("\n");
    let truncated = end_line < total_lines;

    create_result(
        tool_name,
        file_path,
        output,
        total_lines,
        start_line,
        end_line,
        truncated,
    )
}

/// Create an empty file result
fn create_empty_result(tool_name: &str, file_path: &str) -> ToolResult {
    ToolResult::success("", tool_name, "")
        .with_metadata(
            "file_path",
            serde_json::Value::String(file_path.to_string()),
        )
        .with_metadata("total_lines", serde_json::Value::Number(0.into()))
        .with_metadata("lines_read", serde_json::Value::Number(0.into()))
        .with_metadata("start_line", serde_json::Value::Number(0.into()))
        .with_metadata("end_line", serde_json::Value::Number(0.into()))
        .with_metadata("truncated", serde_json::Value::Bool(false))
}

/// Create a result with metadata
fn create_result(
    tool_name: &str,
    file_path: &str,
    output: String,
    total_lines: usize,
    start_line: usize,
    end_line: usize,
    truncated: bool,
) -> Result<ToolResult, ToolError> {
    let mut result = ToolResult::success("", tool_name, output);

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
