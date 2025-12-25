//! Core execution logic for multi-edit operations

use sage_core::tools::base::ToolError;
use sage_core::tools::types::ToolResult;
use std::path::PathBuf;
use tokio::fs;

use super::types::{EditOperation, truncate_for_display};

/// Perform multiple edits on a file
pub async fn execute_multi_edit(
    file_path: &str,
    edits: Vec<EditOperation>,
    working_directory: &PathBuf,
    read_files: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<PathBuf>>>,
    tool_name: &str,
) -> Result<ToolResult, ToolError> {
    // Resolve and validate path
    let path = if std::path::Path::new(file_path).is_absolute() {
        PathBuf::from(file_path)
    } else {
        working_directory.join(file_path)
    };

    // Security check - using the FileSystemTool trait's is_safe_path
    // We need to implement this check inline since we don't have the tool instance here
    if !is_safe_path(&path, working_directory) {
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
    if !has_been_read(&path, read_files) {
        return Err(ToolError::ValidationFailed(format!(
            "File has not been read: {}. You must use the Read tool first to examine the file before editing it.",
            file_path
        )));
    }

    // Read the file content
    let mut content = fs::read_to_string(&path).await.map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to read file content from '{}': {}",
            file_path, e
        ))
    })?;

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
    fs::write(&path, &content).await.map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to write updated content to file '{}': {}",
            file_path, e
        ))
    })?;

    // Format the success message
    let summary = format!(
        "Successfully applied {} edit(s) to {} ({} total replacement(s)):\n{}",
        edits.len(),
        file_path,
        total_replacements,
        edit_results.join("\n")
    );

    Ok(ToolResult::success("", tool_name, summary))
}

/// Check if a file has been read in this session
fn has_been_read(
    path: &PathBuf,
    read_files: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<PathBuf>>>,
) -> bool {
    if let Ok(files) = read_files.lock() {
        files.contains(path)
    } else {
        false
    }
}

/// Basic path safety check
fn is_safe_path(path: &PathBuf, working_directory: &PathBuf) -> bool {
    // Ensure the path doesn't escape the working directory using .. or absolute paths
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let canonical_working_dir = match working_directory.canonicalize() {
        Ok(p) => p,
        Err(_) => return true, // If we can't canonicalize working dir, allow the operation
    };

    canonical_path.starts_with(canonical_working_dir)
}
