//! Binary file detection and handling

use sage_core::tools::base::ToolError;
use sage_core::tools::types::ToolResult;
use std::path::Path;
use tokio::fs;

/// Detect and handle binary files by extension
pub async fn handle_binary_file(
    path: &Path,
    file_path: &str,
    tool_name: &str,
) -> Result<Option<ToolResult>, ToolError> {
    let metadata = fs::metadata(path).await.map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to read file metadata for '{}': {}. Ensure the file exists and you have permission to access it.",
            file_path, e
        ))
    })?;

    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            // Image formats
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "svg" => {
                return Ok(Some(ToolResult::success(
                    "",
                    tool_name,
                    format!(
                        "[Image file detected: {}]\n\nThis is a {} image file. Binary content cannot be displayed as text.\nFile size: {} bytes",
                        file_path,
                        ext_str.to_uppercase(),
                        metadata.len()
                    ),
                )));
            }
            // PDF format
            "pdf" => {
                return Ok(Some(ToolResult::success(
                    "",
                    tool_name,
                    format!(
                        "[PDF file detected: {}]\n\nThis is a PDF file. Binary content cannot be displayed as text.\nFile size: {} bytes\n\nTo extract text from PDF, consider using a dedicated PDF processing tool.",
                        file_path,
                        metadata.len()
                    ),
                )));
            }
            // Other binary formats
            "exe" | "dll" | "so" | "dylib" | "bin" | "zip" | "tar" | "gz" | "rar" | "7z" => {
                return Ok(Some(ToolResult::success(
                    "",
                    tool_name,
                    format!(
                        "[Binary file detected: {}]\n\nThis is a binary {} file. Content cannot be displayed as text.\nFile size: {} bytes",
                        file_path,
                        ext_str.to_uppercase(),
                        metadata.len()
                    ),
                )));
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Create a binary file result for non-UTF8 data
pub fn create_binary_result(file_path: &str, file_size: u64, tool_name: &str) -> ToolResult {
    ToolResult::success(
        "",
        tool_name,
        format!(
            "[Binary file detected: {}]\n\nFile contains non-UTF8 data and cannot be displayed as text.\nFile size: {} bytes",
            file_path, file_size
        ),
    )
}
