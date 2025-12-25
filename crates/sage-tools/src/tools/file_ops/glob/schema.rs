//! Schema and validation for the glob tool

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use tracing::instrument;

use super::types::GlobTool;

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

    #[instrument(skip(self, call), fields(call_id = %call.id, pattern = call.get_string("pattern").as_deref().unwrap_or("<missing>")))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let pattern = call.get_string("pattern").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'pattern' parameter".to_string())
        })?;

        let path = call.get_string("path");
        let path_ref = path.as_deref();

        let mut result = self.find_files(&pattern, path_ref).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to find files matching pattern '{}': {}",
                pattern, e
            ))
        })?;
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
