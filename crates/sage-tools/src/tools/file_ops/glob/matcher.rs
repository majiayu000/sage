//! File matching logic for glob patterns

use glob::glob as glob_pattern;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::ToolResult;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use super::types::{GlobTool, MAX_FILES};

impl GlobTool {
    /// Find files matching a glob pattern
    pub(crate) async fn find_files(
        &self,
        pattern: &str,
        search_path: Option<&str>,
    ) -> Result<ToolResult, ToolError> {
        // Determine the search directory
        let base_path = if let Some(path_str) = search_path {
            let path = self.resolve_path(path_str);

            // Security check
            if !self.is_safe_path(&path) {
                return Err(ToolError::PermissionDenied(format!(
                    "Access denied to path: {}",
                    path.display()
                )));
            }

            // Verify directory exists
            if !path.exists() {
                return Err(ToolError::ExecutionFailed(format!(
                    "Directory does not exist: {}",
                    path_str
                )));
            }

            if !path.is_dir() {
                return Err(ToolError::ExecutionFailed(format!(
                    "Path is not a directory: {}",
                    path_str
                )));
            }

            path
        } else {
            self.working_directory.clone()
        };

        // Construct the full glob pattern
        let full_pattern = base_path.join(pattern);
        let pattern_str = full_pattern
            .to_str()
            .ok_or_else(|| ToolError::ExecutionFailed("Invalid path encoding".to_string()))?;

        // Execute glob pattern matching
        let mut matches: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in glob_pattern(pattern_str)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid glob pattern: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    // Security check for each matched file
                    if !self.is_safe_path(&path) {
                        continue; // Skip files outside safe paths
                    }

                    // Get modification time for sorting
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            matches.push((path, modified));
                        } else {
                            // If we can't get modified time, use UNIX_EPOCH as fallback
                            matches.push((path, SystemTime::UNIX_EPOCH));
                        }
                    }

                    // Limit the number of results
                    if matches.len() >= MAX_FILES {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("Error reading glob entry: {}", e);
                    continue;
                }
            }
        }

        // Sort by modification time (newest first)
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        // Extract just the paths
        let file_paths: Vec<String> = matches
            .into_iter()
            .map(|(path, _)| {
                // Try to make paths relative to working directory for cleaner output
                if let Ok(rel_path) = path.strip_prefix(&self.working_directory) {
                    rel_path.to_string_lossy().to_string()
                } else {
                    path.to_string_lossy().to_string()
                }
            })
            .collect();

        let file_count = file_paths.len();
        let truncated = file_count >= MAX_FILES;

        // Format output
        let mut output = if file_count == 0 {
            format!("No files found matching pattern: {}", pattern)
        } else {
            let mut result = format!(
                "Found {} file{} matching pattern '{}'{}:\n\n",
                file_count,
                if file_count == 1 { "" } else { "s" },
                pattern,
                if truncated {
                    format!(" (limited to first {})", MAX_FILES)
                } else {
                    String::new()
                }
            );

            for (i, path) in file_paths.iter().enumerate() {
                result.push_str(&format!("{}. {}\n", i + 1, path));
            }

            result
        };

        // Add search path info if specified
        if let Some(path_str) = search_path {
            output = format!("Search directory: {}\n\n{}", path_str, output);
        }

        // Build result with metadata
        let mut result = ToolResult::success("", self.name(), output)
            .with_metadata("pattern", serde_json::Value::String(pattern.to_string()))
            .with_metadata(
                "results_count",
                serde_json::Value::Number(file_count.into()),
            )
            .with_metadata("truncated", serde_json::Value::Bool(truncated));

        if let Some(path_str) = search_path {
            result = result.with_metadata(
                "search_path",
                serde_json::Value::String(path_str.to_string()),
            );
        }

        Ok(result)
    }
}

impl FileSystemTool for GlobTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
