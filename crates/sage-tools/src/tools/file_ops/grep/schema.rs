//! Tool trait implementation for grep

use super::output::GrepOutputMode;
use super::params;
use crate::tools::file_ops::grep::GrepTool;
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use tracing::instrument;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        r#"A powerful search tool built on ripgrep

  Usage:
  - ALWAYS use Grep for search tasks. NEVER invoke `grep` or `rg` as a Bash command. The Grep tool has been optimized for correct permissions and access.
  - Supports full regex syntax (e.g., "log.*Error", "function\s+\w+")
  - Filter files with glob parameter (e.g., "*.js", "**/*.tsx") or type parameter (e.g., "js", "py", "rust")
  - Output modes: "content" shows matching lines, "files_with_matches" shows only file paths (default), "count" shows match counts
  - Use Task tool for open-ended searches requiring multiple rounds
  - Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping (use `interface\{\}` to find `interface{}` in Go code)
  - Multiline matching: By default patterns match within single lines only. For cross-line patterns like `struct \{[\s\S]*?field`, use `multiline: true`

Parameters:
  - pattern: The regular expression pattern to search for in file contents (required)
  - path: File or directory to search in (defaults to current working directory)
  - glob: Glob pattern to filter files (e.g., "*.js", "*.{ts,tsx}")
  - type: File type to search (e.g., "js", "py", "rust") - more efficient than glob for standard types
  - output_mode: "content" (shows matching lines), "files_with_matches" (file paths only, default), "count" (match counts)
  - -A: Number of lines to show after each match (requires output_mode: "content")
  - -B: Number of lines to show before each match (requires output_mode: "content")
  - -C: Number of lines to show before and after each match (requires output_mode: "content")
  - -i: Case insensitive search
  - -n: Show line numbers in output (defaults to true)
  - multiline: Enable multiline mode where . matches newlines and patterns can span lines
  - head_limit: Limit output to first N lines/entries
  - offset: Skip first N lines/entries before applying head_limit"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            params::get_tool_parameters(),
        )
    }

    #[instrument(skip(self, call), fields(call_id = %call.id, pattern = call.get_string("pattern").as_deref().unwrap_or("<missing>")))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let pattern = call.get_string("pattern").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'pattern' parameter".to_string())
        })?;

        let path = call.get_string("path");
        let glob_filter = call.get_string("glob");
        let type_filter = call.get_string("type");

        let output_mode = if let Some(mode) = call.get_string("output_mode") {
            GrepOutputMode::from_str(&mode)?
        } else {
            GrepOutputMode::default()
        };

        let case_insensitive = call.get_bool("-i").unwrap_or(false);
        let show_line_numbers = call.get_bool("-n").unwrap_or(true);

        // Handle context parameters
        let lines_before = call.get_usize("-B", 0);
        let lines_after = call.get_usize("-A", 0);
        let context = call.get_usize("-C", 0);

        // If -C is specified, it overrides -A and -B
        let (lines_before, lines_after) = if context > 0 {
            (context, context)
        } else {
            (lines_before, lines_after)
        };

        let multiline = call.get_bool("multiline").unwrap_or(false);
        let head_limit = call.get_usize("head_limit", 0);
        let offset = call.get_usize("offset", 0);

        let mut result = self
            .search(
                &pattern,
                path.as_deref(),
                glob_filter.as_deref(),
                type_filter.as_deref(),
                case_insensitive,
                show_line_numbers,
                lines_before,
                lines_after,
                multiline,
                output_mode,
                head_limit,
                offset,
            )
            .await?;

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Validate pattern exists
        if call.get_string("pattern").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'pattern' parameter".to_string(),
            ));
        }

        // Validate output_mode if specified
        if let Some(mode) = call.get_string("output_mode") {
            GrepOutputMode::from_str(&mode)?;
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(120)) // 2 minutes for large searches
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Read-only operations can be parallel
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
