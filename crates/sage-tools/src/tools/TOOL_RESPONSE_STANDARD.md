# Tool Response Standardization Guide

## Overview

All tools in Sage Agent must use the standardized `ToolResult` structure for returning execution results. This ensures consistency across the codebase and makes tool responses predictable for both the LLM and UI components.

## Standard ToolResult Structure

Located in `sage-core/src/tools/types.rs`, the `ToolResult` structure includes:

```rust
pub struct ToolResult {
    pub call_id: String,           // Tool call ID this result corresponds to
    pub tool_name: String,          // Name of the tool that was executed
    pub success: bool,              // Whether the tool execution was successful
    pub output: Option<String>,     // Output from the tool (if successful)
    pub error: Option<String>,      // Error message (if failed)
    pub exit_code: Option<i32>,     // Exit code (for command-line tools)
    pub execution_time_ms: Option<u64>, // Execution time in milliseconds
    pub metadata: HashMap<String, serde_json::Value>, // Additional metadata
}
```

## Best Practices

### 1. Always Use Helper Methods

**DO THIS:**
```rust
// Success case
let result = ToolResult::success(&call.id, self.name(), "Operation completed")
    .with_metadata("files_processed", 42)
    .with_execution_time(123);

// Error case
let result = ToolResult::error(&call.id, self.name(), "File not found");
```

**DON'T DO THIS:**
```rust
// Manual construction is error-prone and inconsistent
let result = ToolResult {
    call_id: call.id.clone(),
    tool_name: self.name().to_string(),
    success: true,
    output: Some("Operation completed".to_string()),
    error: None,
    exit_code: Some(0),
    execution_time_ms: None,
    metadata: HashMap::new(),
};
```

### 2. Use Metadata for Structured Data

Keep the `output` field human-readable and use `metadata` for structured information:

```rust
let result = ToolResult::success(&call.id, self.name(), formatted_output)
    .with_metadata("total_lines", total_lines)
    .with_metadata("lines_read", lines_read)
    .with_metadata("file_path", file_path)
    .with_metadata("truncated", truncated);
```

### 3. Set call_id After Construction

Always set the `call_id` from the incoming `ToolCall`:

```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    // ... do work ...

    let mut result = ToolResult::success(&call.id, self.name(), output);
    result.call_id = call.id.clone(); // Ensure correct call_id
    Ok(result)
}
```

### 4. Include Execution Time for Performance Tracking

```rust
let start_time = std::time::Instant::now();

// ... execute operation ...

let result = ToolResult::success(&call.id, self.name(), output)
    .with_execution_time(start_time.elapsed().as_millis() as u64);
```

### 5. Provide Informative Error Messages

Error messages should be clear and actionable:

```rust
// Good error message
return Err(ToolError::ExecutionFailed(format!(
    "File not found: {}. Please verify the path is correct and the file exists.",
    file_path
)));

// Poor error message
return Err(ToolError::ExecutionFailed("Error".to_string()));
```

## Examples

### File Operations Tool

```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let file_path = call.get_string("file_path")
        .ok_or_else(|| ToolError::InvalidArguments("Missing 'file_path' parameter".to_string()))?;

    let start_time = std::time::Instant::now();

    // Read file
    let content = fs::read_to_string(&file_path).await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file '{}': {}", file_path, e)))?;

    let lines = content.lines().count();

    // Build result
    let mut result = ToolResult::success(&call.id, self.name(), content)
        .with_metadata("file_path", file_path)
        .with_metadata("total_lines", lines)
        .with_execution_time(start_time.elapsed().as_millis() as u64);

    result.call_id = call.id.clone();
    Ok(result)
}
```

### Command Execution Tool

```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let command = call.get_string("command")
        .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

    let start_time = std::time::Instant::now();

    let output = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .output()
        .await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

    let execution_time = start_time.elapsed().as_millis() as u64;

    // Format output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let result_text = if stderr.is_empty() {
        stdout.to_string()
    } else {
        format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr)
    };

    // Build result based on success
    let mut result = if output.status.success() {
        ToolResult::success(&call.id, self.name(), result_text)
    } else {
        ToolResult::error(&call.id, self.name(), format!(
            "Command failed with exit code: {:?}\n\n{}",
            output.status.code(),
            result_text
        ))
    };

    result.exit_code = output.status.code();
    result.execution_time_ms = Some(execution_time);
    result = result
        .with_metadata("command", command)
        .with_metadata("exit_code", output.status.code().unwrap_or(-1));

    Ok(result)
}
```

### Network Request Tool

```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let url = call.get_string("url")
        .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

    let start_time = std::time::Instant::now();

    // Make request
    let response = reqwest::get(&url).await
        .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

    let status = response.status();
    let body = response.text().await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

    let execution_time = start_time.elapsed().as_millis() as u64;

    // Build result
    let result = ToolResult::success(&call.id, self.name(), body)
        .with_metadata("url", url)
        .with_metadata("status_code", status.as_u16())
        .with_metadata("content_length", body.len())
        .with_execution_time(execution_time);

    Ok(result)
}
```

## Migration Guide

If you find a tool that's not using the standardized format:

### Before:
```rust
// Old pattern - manual construction
Ok(ToolResult {
    call_id: call.id.clone(),
    tool_name: "my_tool".to_string(),
    success: true,
    output: Some(output),
    error: None,
    exit_code: None,
    execution_time_ms: None,
    metadata: HashMap::new(),
})
```

### After:
```rust
// New pattern - use helper methods
let mut result = ToolResult::success(&call.id, self.name(), output);
result.call_id = call.id.clone();
Ok(result)
```

## Validation Checklist

When implementing a new tool or reviewing an existing one, verify:

- [ ] Uses `ToolResult::success()` for successful operations
- [ ] Uses `ToolResult::error()` for failed operations
- [ ] Sets `call_id` from the incoming `ToolCall`
- [ ] Includes execution time via `.with_execution_time()`
- [ ] Uses `.with_metadata()` for structured data
- [ ] Error messages are clear and actionable
- [ ] Output field contains human-readable text
- [ ] Metadata field contains machine-readable structured data

## Current Status (2025-12-23)

**Audit Results:**
- ✅ All core file operations tools (bash, edit, read, write, grep, glob) are standardized
- ✅ All task management tools (TodoWrite, task_management) are standardized
- ✅ All network tools (web_fetch, web_search, browser, http_client) are standardized
- ✅ All interaction tools (ask_user) are standardized
- ✅ 34+ tools confirmed using standardized format

**Recent Improvements (MED-005 Fix):**
1. **GlobTool** - Enhanced with metadata for:
   - `pattern`: The glob pattern used
   - `results_count`: Number of files found
   - `truncated`: Whether results were limited
   - `search_path`: Search directory (if specified)

2. **GrepTool** - Enhanced with comprehensive metadata:
   - `pattern`: Search pattern used
   - `results_count`: Number of files/results
   - `total_matches`: Total match count
   - `output_mode`: Output format (content/files_with_matches/count)
   - `search_path`, `glob_filter`, `type_filter`: Applied filters
   - `case_insensitive`: Search mode

3. **HttpClientTool** - Complete refactoring:
   - Fixed to use standard Tool trait (schema() and execute(&ToolCall))
   - Changed from `ToolResult::new()` to `ToolResult::success()`
   - Added structured metadata (status, response_time_ms, url, content_type, content_length)
   - Added proper validate() method
   - Standardized error handling with ToolError

**Tools Verified:**
1. BashTool - ✅ Uses ToolResult::success/error with metadata (exit_code, execution_time, command, working_directory)
2. EditTool - ✅ Uses ToolResult::success/error with proper error messages
3. ReadTool - ✅ Uses ToolResult::success with rich metadata (total_lines, lines_read, file_path, truncated)
4. GrepTool - ✅ Uses ToolResult::success with comprehensive metadata (pattern, results_count, total_matches, filters)
5. GlobTool - ✅ Uses ToolResult::success with metadata (pattern, results_count, truncated, search_path)
6. WriteTool - ✅ Uses ToolResult::success with operation tracking
7. TodoWriteTool - ✅ Uses ToolResult::success with metadata
8. WebFetchTool - ✅ Uses ToolResult::success
9. HttpClientTool - ✅ Uses ToolResult::success with metadata (status, response_time_ms, url, content_type)
10. AskUserQuestionTool - ✅ Uses standardized format

## References

- Core types: `sage-core/src/tools/types.rs`
- Base trait: `sage-core/src/tools/base.rs`
- Example implementations: `sage-tools/src/tools/file_ops/`
