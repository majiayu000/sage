//! Search logic for grep tool using ripgrep library
//!
//! This module uses the `grep-searcher` and `ignore` crates from ripgrep
//! for high-performance file searching with proper binary detection.

use super::filters;
use super::output::GrepOutputMode;
use crate::tools::file_ops::grep::GrepTool;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{
    BinaryDetection, SearcherBuilder, Sink, SinkContext, SinkContextKind, SinkMatch,
};
use ignore::WalkBuilder;
use sage_core::tools::base::{FileSystemTool, ToolError};
use sage_core::tools::types::ToolResult;
use std::io;
use std::path::Path;

/// Result from searching a single file
#[derive(Debug, Clone)]
struct FileSearchResult {
    path: String,
    matches: Vec<MatchLine>,
    match_count: usize,
}

/// A single matching line with context
#[derive(Debug, Clone)]
struct MatchLine {
    line_number: Option<u64>,
    content: String,
    // Reserved for future use - distinguishing match vs context lines
    #[allow(dead_code)]
    is_context: bool,
}

/// Custom sink to collect matches with context lines
struct MatchCollector {
    matches: Vec<MatchLine>,
    match_count: usize,
    show_line_numbers: bool,
    collect_content: bool,
}

impl MatchCollector {
    fn new(show_line_numbers: bool, collect_content: bool) -> Self {
        Self {
            matches: Vec::new(),
            match_count: 0,
            show_line_numbers,
            collect_content,
        }
    }
}

impl Sink for MatchCollector {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        self.match_count += 1;

        if self.collect_content {
            let content = String::from_utf8_lossy(mat.bytes()).trim_end().to_string();
            self.matches.push(MatchLine {
                line_number: if self.show_line_numbers {
                    mat.line_number()
                } else {
                    None
                },
                content,
                is_context: false,
            });
        }

        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        if self.collect_content {
            let content = String::from_utf8_lossy(ctx.bytes()).trim_end().to_string();
            let is_context = !matches!(ctx.kind(), SinkContextKind::Other);
            self.matches.push(MatchLine {
                line_number: if self.show_line_numbers {
                    ctx.line_number()
                } else {
                    None
                },
                content,
                is_context,
            });
        }

        Ok(true)
    }

    fn context_break(&mut self, _searcher: &grep_searcher::Searcher) -> Result<bool, Self::Error> {
        if self.collect_content && !self.matches.is_empty() {
            // Add a separator between context groups
            self.matches.push(MatchLine {
                line_number: None,
                content: "--".to_string(),
                is_context: true,
            });
        }
        Ok(true)
    }
}

impl GrepTool {
    /// Search files with the given pattern using ripgrep library
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
        // Build the regex matcher using grep-regex
        let mut matcher_builder = RegexMatcherBuilder::new();
        matcher_builder.case_insensitive(case_insensitive);

        if multiline {
            matcher_builder.multi_line(true);
            matcher_builder.dot_matches_new_line(true);
        }

        let matcher = matcher_builder
            .build(pattern)
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

        // Build the searcher with binary detection
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(show_line_numbers)
            .before_context(lines_before)
            .after_context(lines_after)
            .build();

        // Collect results
        let mut results = Vec::new();

        // If it's a file, search just that file
        if base_path.is_file() {
            if let Some(result) = self.search_single_file(
                &base_path,
                &matcher,
                &searcher,
                show_line_numbers,
                output_mode,
            )? {
                results.push(result);
            }
        } else {
            // Build walker with ignore support
            let mut walk_builder = WalkBuilder::new(&base_path);
            walk_builder
                .hidden(false) // Don't skip hidden files by default
                .git_ignore(true) // Respect .gitignore
                .git_global(true) // Respect global gitignore
                .git_exclude(true) // Respect .git/info/exclude
                .ignore(true) // Respect .ignore files
                .parents(true); // Respect parent directory ignore files

            // Apply glob filter if specified
            if let Some(glob) = glob_filter {
                // Convert simple glob to ignore pattern
                let mut override_builder = ignore::overrides::OverrideBuilder::new(&base_path);
                override_builder
                    .add(&format!("!{}", glob))
                    .map_err(|e| ToolError::InvalidArguments(format!("Invalid glob: {}", e)))?;
                // Invert: include only matching files
                override_builder
                    .add(glob)
                    .map_err(|e| ToolError::InvalidArguments(format!("Invalid glob: {}", e)))?;
                let overrides = override_builder
                    .build()
                    .map_err(|e| ToolError::InvalidArguments(format!("Invalid glob: {}", e)))?;
                walk_builder.overrides(overrides);
            }

            // Walk and search
            for entry in walk_builder.build().filter_map(|e| e.ok()) {
                let path = entry.path();

                // Skip directories
                if !path.is_file() {
                    continue;
                }

                // Apply type filter if specified
                if let Some(file_type) = type_filter {
                    if !filters::matches_type(path, file_type) {
                        continue;
                    }
                }

                // Skip common binary extensions (as additional filter)
                if filters::is_binary_extension(path) {
                    continue;
                }

                // Search the file
                if let Some(result) = self.search_single_file(
                    path,
                    &matcher,
                    &searcher,
                    show_line_numbers,
                    output_mode,
                )? {
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

        // Calculate total matches
        let total_matches: usize = results.iter().map(|r| r.match_count).sum();

        // Format output based on mode
        let output = if results.is_empty() {
            format!("No matches found for pattern: {}", pattern)
        } else {
            match output_mode {
                GrepOutputMode::Content => results
                    .iter()
                    .map(|r| {
                        let lines: Vec<String> = r
                            .matches
                            .iter()
                            .map(|m| {
                                if let Some(line_num) = m.line_number {
                                    format!("{}:\t{}", line_num, m.content)
                                } else {
                                    m.content.clone()
                                }
                            })
                            .collect();
                        format!("{}:\n{}", r.path, lines.join("\n"))
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                GrepOutputMode::FilesWithMatches => {
                    format!(
                        "{}\n\nTotal: {} file(s) with matches",
                        results
                            .iter()
                            .map(|r| r.path.as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                        results.len()
                    )
                }
                GrepOutputMode::Count => {
                    format!(
                        "{}\n\nTotal matches: {}",
                        results
                            .iter()
                            .map(|r| format!("{}:{}", r.path, r.match_count))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        total_matches
                    )
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

    /// Search a single file using grep-searcher
    fn search_single_file(
        &self,
        path: &Path,
        matcher: &grep_regex::RegexMatcher,
        searcher: &grep_searcher::Searcher,
        show_line_numbers: bool,
        output_mode: GrepOutputMode,
    ) -> Result<Option<FileSearchResult>, ToolError> {
        // Clone searcher for this search
        let mut searcher = searcher.clone();

        // Create our custom sink
        let collect_content = output_mode == GrepOutputMode::Content;
        let mut collector = MatchCollector::new(show_line_numbers, collect_content);

        let sink_result = searcher.search_path(matcher, path, &mut collector);

        // Handle search result
        match sink_result {
            Ok(_) => {}
            Err(e) => {
                // Check if it's a binary file detection quit
                if e.to_string().contains("binary") {
                    return Ok(None);
                }
                // For other errors, log and skip
                tracing::debug!("Skipping file {:?}: {}", path, e);
                return Ok(None);
            }
        }

        if collector.match_count == 0 {
            return Ok(None);
        }

        let relative_path = path
            .strip_prefix(self.working_directory())
            .unwrap_or(path)
            .display()
            .to_string();

        // Remove trailing separator if present
        let mut matches = collector.matches;
        if let Some(last) = matches.last() {
            if last.content == "--" {
                matches.pop();
            }
        }

        Ok(Some(FileSearchResult {
            path: relative_path,
            matches,
            match_count: collector.match_count,
        }))
    }
}
