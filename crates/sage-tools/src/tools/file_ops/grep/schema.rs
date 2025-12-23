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
        "A powerful search tool built on regex for finding patterns in files.

Features:
- Full regex pattern matching with multiline support
- Multiple output modes: content, files_with_matches, count
- File filtering by glob pattern or file type
- Context lines (-A, -B, -C) for matches
- Case insensitive search (-i)
- Line numbers (-n)
- Result limiting with head_limit and offset

Common usage:
- Search for pattern: pattern='TODO', output_mode='files_with_matches'
- View matches: pattern='function.*export', output_mode='content', '-n'=true
- Filter files: glob='*.rs' or type='rust'
- Context: '-A'=3, '-B'=3 to show surrounding lines

Automatically skips:
- Binary files and common cache directories (node_modules, target, .git, etc.)
- Binary file extensions (images, videos, archives, etc.)
- Hidden files"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), params::get_tool_parameters())
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
        let lines_before = call.get_number("-B").unwrap_or(0.0) as usize;
        let lines_after = call.get_number("-A").unwrap_or(0.0) as usize;
        let context = call.get_number("-C").unwrap_or(0.0) as usize;

        // If -C is specified, it overrides -A and -B
        let (lines_before, lines_after) = if context > 0 {
            (context, context)
        } else {
            (lines_before, lines_after)
        };

        let multiline = call.get_bool("multiline").unwrap_or(false);
        let head_limit = call.get_number("head_limit").unwrap_or(0.0) as usize;
        let offset = call.get_number("offset").unwrap_or(0.0) as usize;

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

    fn max_execution_time(&self) -> Option<u64> {
        Some(120) // 2 minutes for large searches
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Read-only operations can be parallel
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
