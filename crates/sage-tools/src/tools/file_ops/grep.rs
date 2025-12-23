//! Grep tool for searching file contents with regex patterns

use async_trait::async_trait;
use regex::{Regex, RegexBuilder};
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::instrument;
use walkdir::WalkDir;

/// Output mode for grep results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrepOutputMode {
    /// Show matching lines with content
    Content,
    /// Show only file paths with matches
    FilesWithMatches,
    /// Show match counts per file
    Count,
}

impl GrepOutputMode {
    fn from_str(s: &str) -> Result<Self, ToolError> {
        match s {
            "content" => Ok(Self::Content),
            "files_with_matches" => Ok(Self::FilesWithMatches),
            "count" => Ok(Self::Count),
            _ => Err(ToolError::InvalidArguments(format!(
                "Invalid output_mode: {}. Use 'content', 'files_with_matches', or 'count'",
                s
            ))),
        }
    }
}

impl Default for GrepOutputMode {
    fn default() -> Self {
        Self::FilesWithMatches
    }
}

/// Tool for searching files using regex patterns (like ripgrep)
pub struct GrepTool {
    working_directory: PathBuf,
}

impl GrepTool {
    /// Create a new grep tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a grep tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }

    /// Check if a file should be included based on glob pattern
    fn matches_glob(path: &Path, glob_pattern: &str) -> bool {
        if let Ok(pattern) = glob::Pattern::new(glob_pattern) {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                return pattern.matches(file_name);
            }
        }
        false
    }

    /// Get file extension for type filtering
    fn get_extension(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
    }

    /// Check if a file matches the type filter
    fn matches_type(path: &Path, type_filter: &str) -> bool {
        if let Some(ext) = Self::get_extension(path) {
            match type_filter {
                "rs" | "rust" => ext == "rs",
                "js" | "javascript" => matches!(ext.as_str(), "js" | "jsx" | "mjs" | "cjs"),
                "ts" | "typescript" => matches!(ext.as_str(), "ts" | "tsx"),
                "py" | "python" => ext == "py",
                "go" => ext == "go",
                "java" => ext == "java",
                "c" => ext == "c",
                "cpp" | "c++" => matches!(ext.as_str(), "cpp" | "cc" | "cxx" | "hpp" | "h"),
                "rb" | "ruby" => ext == "rb",
                "php" => ext == "php",
                "html" => matches!(ext.as_str(), "html" | "htm"),
                "css" => ext == "css",
                "json" => ext == "json",
                "yaml" | "yml" => matches!(ext.as_str(), "yaml" | "yml"),
                "xml" => ext == "xml",
                "md" | "markdown" => matches!(ext.as_str(), "md" | "markdown"),
                "txt" | "text" => ext == "txt",
                "toml" => ext == "toml",
                "sql" => ext == "sql",
                "sh" | "shell" | "bash" => matches!(ext.as_str(), "sh" | "bash" | "zsh"),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Search files with the given pattern
    async fn search(
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
            self.working_directory.clone()
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
                    if !Self::matches_glob(path, glob) {
                        continue;
                    }
                }

                if let Some(file_type) = type_filter {
                    if !Self::matches_type(path, file_type) {
                        continue;
                    }
                }

                // Skip binary files and common ignore patterns
                if self.should_skip_file(path) {
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
        let mode_str = match output_mode {
            GrepOutputMode::Content => "content",
            GrepOutputMode::FilesWithMatches => "files_with_matches",
            GrepOutputMode::Count => "count",
        };

        let mut result = ToolResult::success("", self.name(), output)
            .with_metadata("pattern", serde_json::Value::String(pattern.to_string()))
            .with_metadata("results_count", serde_json::Value::Number(results.len().into()))
            .with_metadata("total_matches", serde_json::Value::Number(total_matches.into()))
            .with_metadata("output_mode", serde_json::Value::String(mode_str.to_string()));

        if let Some(path) = search_path {
            result = result.with_metadata("search_path", serde_json::Value::String(path.to_string()));
        }

        if let Some(glob) = glob_filter {
            result = result.with_metadata("glob_filter", serde_json::Value::String(glob.to_string()));
        }

        if let Some(file_type) = type_filter {
            result = result.with_metadata("type_filter", serde_json::Value::String(file_type.to_string()));
        }

        if case_insensitive {
            result = result.with_metadata("case_insensitive", serde_json::Value::Bool(true));
        }

        Ok(result)
    }

    /// Search a single file
    fn search_file(
        &self,
        path: &Path,
        regex: &Regex,
        show_line_numbers: bool,
        lines_before: usize,
        lines_after: usize,
        output_mode: GrepOutputMode,
    ) -> Result<Option<String>, ToolError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            // Skip files that can't be read as text (likely binary)
            if e.kind() == std::io::ErrorKind::InvalidData {
                return ToolError::Other("Binary file".to_string());
            }
            ToolError::Io(e)
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let mut matching_lines = Vec::new();
        let mut match_count = 0;

        for (i, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                match_count += 1;

                if output_mode == GrepOutputMode::Content {
                    // Add context lines before
                    let start = i.saturating_sub(lines_before);

                    for (idx, ctx_line) in lines[start..i].iter().enumerate() {
                        let line_num = start + idx + 1;
                        if show_line_numbers {
                            matching_lines.push(format!("{}:\t{}", line_num, ctx_line));
                        } else {
                            matching_lines.push(ctx_line.to_string());
                        }
                    }

                    // Add the matching line
                    if show_line_numbers {
                        matching_lines.push(format!("{}:\t{}", i + 1, line));
                    } else {
                        matching_lines.push(line.to_string());
                    }

                    // Add context lines after
                    let end = std::cmp::min(i + lines_after + 1, lines.len());
                    for (idx, ctx_line) in lines[(i + 1)..end].iter().enumerate() {
                        let line_num = i + 2 + idx;
                        if show_line_numbers {
                            matching_lines.push(format!("{}:\t{}", line_num, ctx_line));
                        } else {
                            matching_lines.push(ctx_line.to_string());
                        }
                    }

                    if lines_before > 0 || lines_after > 0 {
                        matching_lines.push("--".to_string());
                    }
                }
            }
        }

        if match_count == 0 {
            return Ok(None);
        }

        let relative_path = path.strip_prefix(&self.working_directory).unwrap_or(path);

        let result = match output_mode {
            GrepOutputMode::Content => {
                format!(
                    "{}:\n{}",
                    relative_path.display(),
                    matching_lines.join("\n")
                )
            }
            GrepOutputMode::FilesWithMatches => relative_path.display().to_string(),
            GrepOutputMode::Count => {
                format!("{}:{}", relative_path.display(), match_count)
            }
        };

        Ok(Some(result))
    }

    /// Check if a file should be skipped
    fn should_skip_file(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Skip common binary and cache directories
            if path.ancestors().any(|p| {
                if let Some(dir_name) = p.file_name().and_then(|n| n.to_str()) {
                    matches!(
                        dir_name,
                        "node_modules"
                            | "target"
                            | ".git"
                            | ".svn"
                            | ".hg"
                            | "dist"
                            | "build"
                            | "__pycache__"
                            | ".pytest_cache"
                            | ".tox"
                            | "venv"
                            | ".venv"
                    )
                } else {
                    false
                }
            }) {
                return true;
            }

            // Skip common binary extensions
            if let Some(ext) = Self::get_extension(path) {
                if matches!(
                    ext.as_str(),
                    "exe"
                        | "dll"
                        | "so"
                        | "dylib"
                        | "a"
                        | "o"
                        | "obj"
                        | "bin"
                        | "dat"
                        | "db"
                        | "sqlite"
                        | "png"
                        | "jpg"
                        | "jpeg"
                        | "gif"
                        | "ico"
                        | "svg"
                        | "pdf"
                        | "zip"
                        | "tar"
                        | "gz"
                        | "bz2"
                        | "xz"
                        | "rar"
                        | "7z"
                        | "mp3"
                        | "mp4"
                        | "avi"
                        | "mov"
                        | "woff"
                        | "woff2"
                        | "ttf"
                        | "eot"
                ) {
                    return true;
                }
            }

            // Skip hidden files starting with .
            if name.starts_with('.') && name.len() > 1 {
                return true;
            }
        }

        false
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("pattern", "The regex pattern to search for"),
                ToolParameter::optional_string(
                    "path",
                    "File or directory to search (default: current directory)",
                ),
                ToolParameter::optional_string("glob", "Filter files by glob pattern (e.g., '*.rs', '**/*.ts')"),
                ToolParameter::optional_string(
                    "type",
                    "Filter by file type: rs, js, ts, py, go, java, c, cpp, rb, php, html, css, json, yaml, xml, md, txt, toml, sql, sh",
                ),
                ToolParameter::optional_string(
                    "output_mode",
                    "Output mode: 'content' (matching lines), 'files_with_matches' (file paths), 'count' (match counts). Default: 'files_with_matches'",
                ),
                ToolParameter {
                    name: "-i".to_string(),
                    description: "Case insensitive search".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    default: Some(serde_json::json!(false)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "-n".to_string(),
                    description: "Show line numbers (only for output_mode='content')".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    default: Some(serde_json::json!(true)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "-B".to_string(),
                    description: "Lines to show before each match (only for output_mode='content')".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    default: Some(serde_json::json!(0)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "-A".to_string(),
                    description: "Lines to show after each match (only for output_mode='content')".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    default: Some(serde_json::json!(0)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "-C".to_string(),
                    description: "Lines of context (before and after) for each match (only for output_mode='content')".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    default: Some(serde_json::json!(0)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "multiline".to_string(),
                    description: "Enable multiline mode where . matches newlines".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    default: Some(serde_json::json!(false)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "head_limit".to_string(),
                    description: "Limit output to first N results (0 = unlimited)".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    default: Some(serde_json::json!(0)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
                ToolParameter {
                    name: "offset".to_string(),
                    description: "Skip first N results".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    default: Some(serde_json::json!(0)),
                    enum_values: None,
                    properties: HashMap::new(),
                },
            ],
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

impl FileSystemTool for GrepTool {
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
    async fn test_grep_basic_search() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");

        fs::write(&file1, "Hello World\nThis is a test\nAnother line")
            .await
            .unwrap();
        fs::write(&file2, "No match here\nJust some text")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-1",
            "Grep",
            json!({
                "pattern": "test",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("test1.txt"));
        assert!(!result.output.as_ref().unwrap().contains("test2.txt"));
    }

    #[tokio::test]
    async fn test_grep_content_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.rs");

        fs::write(
            &file,
            "fn main() {\n    println!(\"Hello\");\n}\n\nfn test() {\n    println!(\"Test\");\n}",
        )
        .await
        .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-2",
            "Grep",
            json!({
                "pattern": "fn.*\\(\\)",
                "output_mode": "content",
                "-n": true
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("fn main()"));
        assert!(output.contains("fn test()"));
        assert!(output.contains("1:")); // Line numbers
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");

        fs::write(&file, "Hello World\nhello world\nHELLO WORLD")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-3",
            "Grep",
            json!({
                "pattern": "hello",
                "-i": true,
                "output_mode": "count"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("3")); // Should match all 3 lines
    }

    #[tokio::test]
    async fn test_grep_with_context() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");

        fs::write(&file, "line 1\nline 2\nMATCH\nline 4\nline 5")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "Grep",
            json!({
                "pattern": "MATCH",
                "output_mode": "content",
                "-A": 1,
                "-B": 1,
                "-n": true
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("line 2"));
        assert!(output.contains("MATCH"));
        assert!(output.contains("line 4"));
    }

    #[tokio::test]
    async fn test_grep_glob_filter() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("test.rs");
        let txt_file = temp_dir.path().join("test.txt");

        fs::write(&rust_file, "fn main() {}").await.unwrap();
        fs::write(&txt_file, "fn main() {}").await.unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-5",
            "Grep",
            json!({
                "pattern": "fn",
                "glob": "*.rs",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("test.rs"));
        assert!(!output.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_grep_type_filter() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("test.rs");
        let py_file = temp_dir.path().join("test.py");

        fs::write(&rust_file, "fn main() {}").await.unwrap();
        fs::write(&py_file, "def main():").await.unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-6",
            "Grep",
            json!({
                "pattern": "main",
                "type": "rust",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("test.rs"));
        assert!(!output.contains("test.py"));
    }

    #[tokio::test]
    async fn test_grep_head_limit() {
        let temp_dir = TempDir::new().unwrap();

        for i in 1..=5 {
            let file = temp_dir.path().join(format!("test{}.txt", i));
            fs::write(&file, "MATCH").await.unwrap();
        }

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-7",
            "Grep",
            json!({
                "pattern": "MATCH",
                "output_mode": "files_with_matches",
                "head_limit": 3
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        let line_count = output.lines().filter(|l| l.contains("test")).count();
        assert_eq!(line_count, 3); // Should only show 3 files
    }

    #[tokio::test]
    async fn test_grep_invalid_regex() {
        let temp_dir = TempDir::new().unwrap();
        let tool = GrepTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-8",
            "Grep",
            json!({
                "pattern": "[invalid(regex",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");

        fs::write(&file, "Some content\nNo matches here")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-9",
            "Grep",
            json!({
                "pattern": "nonexistent",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("No matches found"));
    }

    #[test]
    fn test_grep_schema() {
        let tool = GrepTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Grep");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_matches_type() {
        let path = Path::new("test.rs");
        assert!(GrepTool::matches_type(path, "rust"));
        assert!(GrepTool::matches_type(path, "rs"));
        assert!(!GrepTool::matches_type(path, "python"));

        let path = Path::new("test.tsx");
        assert!(GrepTool::matches_type(path, "typescript"));
        assert!(GrepTool::matches_type(path, "ts"));
    }
}
