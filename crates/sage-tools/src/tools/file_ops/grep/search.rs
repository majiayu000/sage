//! Search logic for grep tool

use super::filters;
use super::output::GrepOutputMode;
use crate::tools::file_ops::grep::GrepTool;
use regex::{Regex, RegexBuilder};
use sage_core::tools::base::{FileSystemTool, ToolError};
use sage_core::tools::types::ToolResult;
use std::path::Path;
use walkdir::WalkDir;

impl GrepTool {
    /// Search files with the given pattern
    pub async fn search(
        &self,
        pattern: &str,
        search_path: Option<&str>,
        glob_filter: Option<&str>,
        type_filter: Option<&str>,
        case_insensitive: bool,
        show_line_numbers: bool,
        lines_before: usize,
        lines_after: usize,
        multiline: bool,
        output_mode: GrepOutputMode,
        head_limit: usize,
        offset: usize,
    ) -> Result<ToolResult, ToolError> {
        // Build the regex pattern
        let mut regex_builder = RegexBuilder::new(pattern);
        regex_builder.case_insensitive(case_insensitive);

        if multiline {
            regex_builder.multi_line(true);
            regex_builder.dot_matches_new_line(true);
        }

        let regex = regex_builder
            .build()
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid regex pattern: {}", e)))?;

        // Resolve search path
        let base_path = if let Some(path) = search_path {
            let resolved = self.resolve_path(path);
            if !resolved.exists() {
                return Err(ToolError::ExecutionFailed(format!(
                    "Path does not exist: {}",
                    path
                )));
            }
            resolved
        } else {
            self.working_directory().to_path_buf()
        };

        // Security check
        if !self.is_safe_path(&base_path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                base_path.display()
            )));
        }

        let mut results = Vec::new();
        let mut total_matches = 0;

        // If it's a file, search just that file
        if base_path.is_file() {
            if let Some(result) = self.search_file(
                &base_path,
                &regex,
                show_line_numbers,
                lines_before,
                lines_after,
                output_mode,
            )? {
                results.push(result);
                total_matches += 1;
            }
        } else {
            // Walk directory
            for entry in WalkDir::new(&base_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                // Skip directories
                if !path.is_file() {
                    continue;
                }

                // Apply filters
                if let Some(glob) = glob_filter {
                    if !filters::matches_glob(path, glob) {
                        continue;
                    }
                }

                if let Some(file_type) = type_filter {
                    if !filters::matches_type(path, file_type) {
                        continue;
                    }
                }

                // Skip binary files and common ignore patterns
                if filters::should_skip_file(path) {
                    continue;
                }

                // Search the file
                if let Some(result) = self.search_file(
                    path,
                    &regex,
                    show_line_numbers,
                    lines_before,
                    lines_after,
                    output_mode,
                )? {
                    total_matches += 1;
                    results.push(result);
                }
            }
        }

        // Apply offset and head_limit
        let results_iter = results.into_iter().skip(offset);
        let results: Vec<_> = if head_limit > 0 {
            results_iter.take(head_limit).collect()
        } else {
            results_iter.collect()
        };

        // Format output based on mode
        let output = if results.is_empty() {
            format!("No matches found for pattern: {}", pattern)
        } else {
            match output_mode {
                GrepOutputMode::Content => results.join("\n\n"),
                GrepOutputMode::FilesWithMatches => {
                    format!(
                        "{}\n\nTotal: {} file(s) with matches",
                        results.join("\n"),
                        results.len()
                    )
                }
                GrepOutputMode::Count => {
                    format!("{}\n\nTotal matches: {}", results.join("\n"), total_matches)
                }
            }
        };

        // Build result with metadata
        let mut result = ToolResult::success("", "Grep", output)
            .with_metadata("pattern", serde_json::Value::String(pattern.to_string()))
            .with_metadata(
                "results_count",
                serde_json::Value::Number(results.len().into()),
            )
            .with_metadata(
                "total_matches",
                serde_json::Value::Number(total_matches.into()),
            )
            .with_metadata(
                "output_mode",
                serde_json::Value::String(output_mode.as_str().to_string()),
            );

        if let Some(path) = search_path {
            result =
                result.with_metadata("search_path", serde_json::Value::String(path.to_string()));
        }

        if let Some(glob) = glob_filter {
            result =
                result.with_metadata("glob_filter", serde_json::Value::String(glob.to_string()));
        }

        if let Some(file_type) = type_filter {
            result = result.with_metadata(
                "type_filter",
                serde_json::Value::String(file_type.to_string()),
            );
        }

        if case_insensitive {
            result = result.with_metadata("case_insensitive", serde_json::Value::Bool(true));
        }

        Ok(result)
    }
}
