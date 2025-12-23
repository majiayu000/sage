//! Fast file pattern matching tool using glob patterns

use async_trait::async_trait;
use glob::glob as glob_pattern;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Maximum number of files to return
const MAX_FILES: usize = 1000;

/// Tool for finding files using glob patterns
pub struct GlobTool {
    working_directory: PathBuf,
}

impl GlobTool {
    /// Create a new glob tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a glob tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }

    /// Find files matching a glob pattern
    async fn find_files(
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

        Ok(ToolResult::success("", self.name(), output))
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "Fast file pattern matching tool for finding files by name patterns.

Supports standard glob patterns:
- * matches any sequence of characters (except /)
- ** matches any sequence of characters (including /)
- ? matches any single character
- [abc] matches any character in the set
- [a-z] matches any character in the range

Examples:
- \"**/*.rs\" - Find all Rust files recursively
- \"src/**/*.ts\" - Find all TypeScript files in src directory
- \"test_*.py\" - Find all Python test files in current directory
- \"*.js\" - Find all JavaScript files
- \"[A-Z]*.md\" - Find all markdown files starting with uppercase letter
- \"src/*/main.rs\" - Find main.rs files one level deep in src

Note: Brace expansion (e.g., *.{js,ts}) may not be supported on all systems.
Use separate glob calls if you need to match multiple extensions.

Results are sorted by modification time (newest first) and limited to 1000 files.
File paths are returned relative to the working directory when possible."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "pattern",
                    "Glob pattern to match files (e.g., \"**/*.rs\", \"src/**/*.ts\")",
                ),
                ToolParameter::optional_string(
                    "path",
                    "Directory to search in (default: current working directory)",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let pattern = call.get_string("pattern").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'pattern' parameter".to_string())
        })?;

        let path = call.get_string("path");
        let path_ref = path.as_deref();

        let mut result = self.find_files(&pattern, path_ref).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to find files matching pattern '{}': {}", pattern, e)))?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let _pattern = call.get_string("pattern").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'pattern' parameter".to_string())
        })?;

        // Validate path if provided
        if let Some(path) = call.get_string("path") {
            if path.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "Path parameter cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(30) // 30 seconds for file searching
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Read-only operation, safe for parallel execution
    }

    fn is_read_only(&self) -> bool {
        true // Glob only reads file metadata, no modifications
    }
}

impl FileSystemTool for GlobTool {
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
    async fn test_glob_tool_find_rust_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("lib.rs"), "pub mod test;")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test.txt"), "test")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-1",
            "Glob",
            json!({
                "pattern": "*.rs"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("main.rs"));
        assert!(output.contains("lib.rs"));
        assert!(!output.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_glob_tool_recursive_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directory structure
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).await.unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}")
            .await
            .unwrap();

        let module_dir = src_dir.join("module");
        fs::create_dir(&module_dir).await.unwrap();
        fs::write(module_dir.join("lib.rs"), "pub mod test;")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-2",
            "Glob",
            json!({
                "pattern": "**/*.rs"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("main.rs"));
        assert!(output.contains("lib.rs"));
        assert!(output.contains("Found 2 files"));
    }

    #[tokio::test]
    async fn test_glob_tool_with_search_path() {
        let temp_dir = TempDir::new().unwrap();

        // Create subdirectory
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).await.unwrap();
        fs::write(sub_dir.join("file.txt"), "content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("root.txt"), "root")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-3",
            "Glob",
            json!({
                "pattern": "*.txt",
                "path": "subdir"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("file.txt"));
        assert!(!output.contains("root.txt"));
        assert!(output.contains("Search directory: subdir"));
    }

    #[tokio::test]
    async fn test_glob_tool_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.txt"), "test")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "Glob",
            json!({
                "pattern": "*.rs"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("No files found"));
    }

    #[tokio::test]
    async fn test_glob_tool_wildcard_extensions() {
        let temp_dir = TempDir::new().unwrap();

        // Create files with different extensions
        fs::write(temp_dir.path().join("file1.js"), "js content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("file2.ts"), "ts content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("file3.py"), "py content")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        // Note: brace expansion {js,ts} may not be supported by all glob implementations
        // so we test with just .js files
        let call = create_tool_call(
            "test-5",
            "Glob",
            json!({
                "pattern": "*.js"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("file1.js"));
        assert!(!output.contains("file2.ts"));
        assert!(!output.contains("file3.py"));
    }

    #[tokio::test]
    async fn test_glob_tool_missing_pattern() {
        let tool = GlobTool::new();

        let call = create_tool_call("test-6", "Glob", json!({}));

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("pattern"));
        }
    }

    #[tokio::test]
    async fn test_glob_tool_invalid_directory() {
        let temp_dir = TempDir::new().unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-7",
            "Glob",
            json!({
                "pattern": "*.txt",
                "path": "nonexistent_directory"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.to_string().contains("does not exist"));
        }
    }

    #[tokio::test]
    async fn test_glob_tool_character_class_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create files with different starting characters
        fs::write(temp_dir.path().join("Apple.txt"), "a")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("Banana.txt"), "b")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("cherry.txt"), "c")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-8",
            "Glob",
            json!({
                "pattern": "[A-Z]*.txt"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("Apple.txt"));
        assert!(output.contains("Banana.txt"));
        assert!(!output.contains("cherry.txt"));
    }

    #[tokio::test]
    async fn test_glob_tool_single_char_wildcard() {
        let temp_dir = TempDir::new().unwrap();

        // Create files with patterns
        fs::write(temp_dir.path().join("test1.txt"), "1")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "2")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test10.txt"), "10")
            .await
            .unwrap();

        let tool = GlobTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-9",
            "Glob",
            json!({
                "pattern": "test?.txt"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let output = result.output.unwrap();
        assert!(output.contains("test1.txt"));
        assert!(output.contains("test2.txt"));
        assert!(!output.contains("test10.txt")); // ? matches single char only
    }

    #[test]
    fn test_glob_tool_schema() {
        let tool = GlobTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Glob");
        assert!(!schema.description.is_empty());
        assert!(schema.description.contains("glob patterns"));
    }

    #[test]
    fn test_glob_tool_validation() {
        let tool = GlobTool::new();

        // Valid call with pattern
        let call = create_tool_call(
            "test-10",
            "Glob",
            json!({
                "pattern": "*.rs"
            }),
        );
        assert!(tool.validate(&call).is_ok());

        // Invalid call without pattern
        let call = create_tool_call("test-11", "Glob", json!({}));
        assert!(tool.validate(&call).is_err());

        // Invalid call with empty path
        let call = create_tool_call(
            "test-12",
            "Glob",
            json!({
                "pattern": "*.rs",
                "path": ""
            }),
        );
        assert!(tool.validate(&call).is_err());
    }

    #[test]
    fn test_glob_tool_properties() {
        let tool = GlobTool::new();

        // Check tool properties
        assert_eq!(tool.name(), "Glob");
        assert!(tool.is_read_only());
        assert!(tool.supports_parallel_execution());
        assert_eq!(tool.max_execution_time(), Some(30));
    }
}
