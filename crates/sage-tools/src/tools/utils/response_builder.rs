//! Tool response builder utilities
//!
//! This module provides helper functions and builders for creating standardized
//! tool responses following the ToolResult format.

use sage_core::tools::types::ToolResult;
use std::time::Instant;

/// Builder for creating standardized file operation responses
pub struct FileOperationResponse {
    start_time: Instant,
    file_path: String,
    operation: String,
}

impl FileOperationResponse {
    pub fn new(file_path: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            file_path: file_path.into(),
            operation: operation.into(),
        }
    }

    pub fn success(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        details: impl Into<String>,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;
        let message = format!("{} {}: {}", self.operation, self.file_path, details.into());

        ToolResult::success(call_id, tool_name, message)
            .with_metadata("file_path", self.file_path)
            .with_metadata("operation", self.operation)
            .with_execution_time(execution_time)
    }

    pub fn with_file_metadata(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        content: impl Into<String>,
        metadata: Vec<(&str, serde_json::Value)>,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;

        let mut result = ToolResult::success(call_id, tool_name, content)
            .with_metadata("file_path", self.file_path.clone())
            .with_execution_time(execution_time);

        for (key, value) in metadata {
            result = result.with_metadata(key, value);
        }

        result
    }
}

/// Builder for creating standardized command execution responses
pub struct CommandResponse {
    start_time: Instant,
    command: String,
    working_directory: Option<String>,
}

impl CommandResponse {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            command: command.into(),
            working_directory: None,
        }
    }

    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn build(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        success: bool,
        output: impl Into<String>,
        exit_code: Option<i32>,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;

        let mut result = if success {
            ToolResult::success(call_id, tool_name, output)
        } else {
            ToolResult::error(call_id, tool_name, output)
        };

        result.exit_code = exit_code;
        result.execution_time_ms = Some(execution_time);

        result = result.with_metadata("command", self.command);

        if let Some(dir) = self.working_directory {
            result = result.with_metadata("working_directory", dir);
        }

        result
    }
}

/// Builder for creating standardized network operation responses
pub struct NetworkResponse {
    start_time: Instant,
    url: String,
    method: String,
}

impl NetworkResponse {
    pub fn new(url: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            url: url.into(),
            method: method.into(),
        }
    }

    pub fn success(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        body: impl Into<String>,
        status_code: u16,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;
        let body_str = body.into();
        let content_length = body_str.len();

        ToolResult::success(call_id, tool_name, body_str)
            .with_metadata("url", self.url)
            .with_metadata("method", self.method)
            .with_metadata("status_code", status_code)
            .with_metadata("content_length", content_length)
            .with_execution_time(execution_time)
    }

    pub fn error(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        error_message: impl Into<String>,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;

        ToolResult::error(call_id, tool_name, error_message)
            .with_metadata("url", self.url)
            .with_metadata("method", self.method)
            .with_execution_time(execution_time)
    }
}

/// Builder for creating standardized search operation responses
pub struct SearchResponse {
    start_time: Instant,
    pattern: String,
    search_path: Option<String>,
}

impl SearchResponse {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            pattern: pattern.into(),
            search_path: None,
        }
    }

    pub fn with_search_path(mut self, path: impl Into<String>) -> Self {
        self.search_path = Some(path.into());
        self
    }

    pub fn build(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        results: Vec<String>,
        total_matches: usize,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;
        let results_count = results.len();

        let output = if results.is_empty() {
            format!("No matches found for pattern: {}", self.pattern)
        } else {
            results.join("\n")
        };

        let mut result = ToolResult::success(call_id, tool_name, output)
            .with_metadata("pattern", self.pattern.clone())
            .with_metadata("results_count", results_count)
            .with_metadata("total_matches", total_matches)
            .with_execution_time(execution_time);

        if let Some(path) = self.search_path {
            result = result.with_metadata("search_path", path);
        }

        result
    }

    pub fn no_matches(
        self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> ToolResult {
        let execution_time = self.start_time.elapsed().as_millis() as u64;

        ToolResult::success(
            call_id,
            tool_name,
            format!("No matches found for pattern: {}", self.pattern),
        )
        .with_metadata("pattern", self.pattern)
        .with_metadata("results_count", 0)
        .with_metadata("total_matches", 0)
        .with_execution_time(execution_time)
    }
}

/// Helper function to create a simple success response with execution time
pub fn simple_success(
    call_id: impl Into<String>,
    tool_name: impl Into<String>,
    message: impl Into<String>,
    start_time: Instant,
) -> ToolResult {
    ToolResult::success(call_id, tool_name, message)
        .with_execution_time(start_time.elapsed().as_millis() as u64)
}

/// Helper function to create a simple error response with execution time
pub fn simple_error(
    call_id: impl Into<String>,
    tool_name: impl Into<String>,
    message: impl Into<String>,
    start_time: Instant,
) -> ToolResult {
    ToolResult::error(call_id, tool_name, message)
        .with_execution_time(start_time.elapsed().as_millis() as u64)
}

/// Helper function to add common file metadata
pub fn with_file_info(
    mut result: ToolResult,
    file_path: impl Into<String>,
    total_lines: usize,
    bytes_processed: usize,
) -> ToolResult {
    result = result
        .with_metadata("file_path", file_path.into())
        .with_metadata("total_lines", total_lines)
        .with_metadata("bytes_processed", bytes_processed);
    result
}

/// Helper function to add pagination metadata
pub fn with_pagination(
    mut result: ToolResult,
    offset: usize,
    limit: usize,
    total: usize,
) -> ToolResult {
    result = result
        .with_metadata("offset", offset)
        .with_metadata("limit", limit)
        .with_metadata("total", total)
        .with_metadata("has_more", total > offset + limit);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_operation_response() {
        let response = FileOperationResponse::new("/path/to/file.txt", "Read");
        let result = response.success("call-1", "ReadTool", "50 lines");

        assert!(result.success);
        assert_eq!(result.call_id, "call-1");
        assert_eq!(result.tool_name, "ReadTool");
        assert!(result.output.unwrap().contains("Read /path/to/file.txt"));
        assert_eq!(
            result.metadata.get("file_path").and_then(|v| v.as_str()),
            Some("/path/to/file.txt")
        );
        assert!(result.execution_time_ms.is_some());
    }

    #[test]
    fn test_command_response() {
        let response = CommandResponse::new("echo hello").with_working_directory("/home/user");
        let result = response.build("call-1", "BashTool", true, "hello\n", Some(0));

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(
            result.metadata.get("command").and_then(|v| v.as_str()),
            Some("echo hello")
        );
        assert_eq!(
            result
                .metadata
                .get("working_directory")
                .and_then(|v| v.as_str()),
            Some("/home/user")
        );
        assert!(result.execution_time_ms.is_some());
    }

    #[test]
    fn test_network_response() {
        let response = NetworkResponse::new("https://example.com", "GET");
        let result = response.success("call-1", "WebFetch", "<html>content</html>", 200);

        assert!(result.success);
        assert_eq!(
            result.metadata.get("status_code").and_then(|v| v.as_u64()),
            Some(200)
        );
        assert_eq!(
            result.metadata.get("url").and_then(|v| v.as_str()),
            Some("https://example.com")
        );
        assert!(result.execution_time_ms.is_some());
    }

    #[test]
    fn test_search_response() {
        let response = SearchResponse::new("test.*pattern").with_search_path("/src");
        let results = vec!["match1".to_string(), "match2".to_string()];
        let result = response.build("call-1", "GrepTool", results, 2);

        assert!(result.success);
        assert_eq!(
            result
                .metadata
                .get("results_count")
                .and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            result.metadata.get("pattern").and_then(|v| v.as_str()),
            Some("test.*pattern")
        );
        assert!(result.execution_time_ms.is_some());
    }

    #[test]
    fn test_simple_helpers() {
        let start_time = Instant::now();

        let success_result = simple_success("call-1", "TestTool", "Success message", start_time);
        assert!(success_result.success);
        assert!(success_result.execution_time_ms.is_some());

        let error_result = simple_error("call-2", "TestTool", "Error message", start_time);
        assert!(!error_result.success);
        assert!(error_result.execution_time_ms.is_some());
    }

    #[test]
    fn test_metadata_helpers() {
        let result = ToolResult::success("call-1", "TestTool", "content");
        let result = with_file_info(result, "/path/file.txt", 100, 5000);

        assert_eq!(
            result.metadata.get("total_lines").and_then(|v| v.as_u64()),
            Some(100)
        );
        assert_eq!(
            result
                .metadata
                .get("bytes_processed")
                .and_then(|v| v.as_u64()),
            Some(5000)
        );
    }

    #[test]
    fn test_pagination_helper() {
        let result = ToolResult::success("call-1", "TestTool", "content");
        let result = with_pagination(result, 10, 20, 100);

        assert_eq!(
            result.metadata.get("offset").and_then(|v| v.as_u64()),
            Some(10)
        );
        assert_eq!(
            result.metadata.get("limit").and_then(|v| v.as_u64()),
            Some(20)
        );
        assert_eq!(
            result.metadata.get("total").and_then(|v| v.as_u64()),
            Some(100)
        );
        assert_eq!(
            result.metadata.get("has_more").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
