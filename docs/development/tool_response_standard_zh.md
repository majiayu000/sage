# Tool Response Format Standard

## Overview

This document defines the standardized response format for all tools in Sage Agent. All tools MUST follow this standard to ensure consistency, reliability, and proper integration with the agent system.

## Standard Structure

All tool responses use the `ToolResult` struct defined in `sage-core/src/tools/types.rs`:

```rust
pub struct ToolResult {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
    pub execution_time_ms: Option<u64>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Required Fields

1. **`success: bool`**
   - `true` if the tool execution succeeded
   - `false` if the tool execution failed
   - MUST be set for all responses

2. **`output: Option<String>`**
   - Contains the primary human-readable output
   - SHOULD be present when `success = true`
   - Contains error details when `success = false` (in addition to `error` field)
   - Use for text that will be shown to the LLM

3. **`metadata: HashMap<String, serde_json::Value>`**
   - Contains structured data about the operation
   - Use for machine-readable information (counts, timestamps, etc.)
   - Examples: `total_lines`, `files_found`, `execution_time`

### Optional Fields

4. **`call_id: String`**
   - Set from the incoming `ToolCall.id`
   - SHOULD be set by the executor after tool returns result

5. **`error: Option<String>`**
   - Human-readable error message
   - SHOULD be present when `success = false`
   - Keep concise and actionable

6. **`exit_code: Option<i32>`**
   - Use for command-line tools (0 = success, non-zero = error)
   - Set automatically by helper methods

7. **`execution_time_ms: Option<u64>`**
   - Execution duration in milliseconds
   - Useful for performance monitoring

## Usage Patterns

### Creating Success Responses

**ALWAYS use the helper method:**

```rust
let result = ToolResult::success(&call.id, self.name(), "Operation completed successfully");
```

**DO NOT manually construct:**

```rust
// ❌ WRONG - Don't do this
let result = ToolResult {
    call_id: call.id.clone(),
    tool_name: self.name().to_string(),
    success: true,
    output: Some("...".to_string()),
    error: None,
    exit_code: Some(0),
    execution_time_ms: None,
    metadata: HashMap::new(),
};
```

### Creating Error Responses

**Use the error helper method:**

```rust
let result = ToolResult::error(&call.id, self.name(), "File not found");
```

### Adding Metadata

**Use the fluent builder pattern:**

```rust
let result = ToolResult::success(&call.id, self.name(), output)
    .with_metadata("total_lines", serde_json::json!(100))
    .with_metadata("files_read", serde_json::json!(5))
    .with_execution_time(execution_time_ms);
```

### Setting Call ID

The `call_id` is typically set after tool execution:

```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let mut result = self.do_work().await?;
    result.call_id = call.id.clone();  // Set call_id from incoming call
    Ok(result)
}
```

## Complete Examples

### Example 1: File Read Tool

```rust
async fn read_file(&self, path: &str) -> Result<ToolResult, ToolError> {
    let content = fs::read_to_string(path).await?;
    let lines = content.lines().count();

    Ok(ToolResult::success("", self.name(), content)
        .with_metadata("file_path", serde_json::Value::String(path.to_string()))
        .with_metadata("total_lines", serde_json::json!(lines))
        .with_metadata("size_bytes", serde_json::json!(content.len())))
}
```

### Example 2: Command Execution Tool

```rust
async fn execute_command(&self, cmd: &str) -> Result<ToolResult, ToolError> {
    let start = std::time::Instant::now();
    let output = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .await?;

    let execution_time = start.elapsed().as_millis() as u64;

    let mut result = if output.status.success() {
        ToolResult::success("", self.name(), String::from_utf8_lossy(&output.stdout))
    } else {
        ToolResult::error("", self.name(), String::from_utf8_lossy(&output.stderr))
    };

    result.exit_code = output.status.code();
    result.execution_time_ms = Some(execution_time);
    result = result.with_metadata("command", serde_json::Value::String(cmd.to_string()));

    Ok(result)
}
```

### Example 3: Search Tool with Metadata

```rust
async fn search(&self, pattern: &str) -> Result<ToolResult, ToolError> {
    let matches = self.find_matches(pattern).await?;
    let total = matches.len();

    let output = format!("Found {} matches:\n{}", total,
                        matches.join("\n"));

    Ok(ToolResult::success("", self.name(), output)
        .with_metadata("pattern", serde_json::Value::String(pattern.to_string()))
        .with_metadata("total_matches", serde_json::json!(total))
        .with_metadata("truncated", serde_json::json!(false)))
}
```

## Best Practices

### 1. Use Helper Methods

✅ **DO:**
```rust
ToolResult::success(&call.id, self.name(), "Done")
ToolResult::error(&call.id, self.name(), "Failed")
```

❌ **DON'T:**
```rust
ToolResult { ... }  // Manual construction
```

### 2. Separate Human-Readable and Machine-Readable Data

✅ **DO:**
```rust
let result = ToolResult::success("", self.name(), "File processed successfully")
    .with_metadata("lines_processed", serde_json::json!(100));
```

❌ **DON'T:**
```rust
let result = ToolResult::success("", self.name(),
    "File processed: 100 lines, 5 errors, 95 success rate");  // Hard to parse
```

### 3. Add Execution Time for Performance-Sensitive Tools

```rust
let start = std::time::Instant::now();
// ... do work ...
let result = ToolResult::success("", self.name(), output)
    .with_execution_time(start.elapsed().as_millis() as u64);
```

### 4. Include Context in Metadata

```rust
result
    .with_metadata("working_directory", serde_json::Value::String(self.working_dir()))
    .with_metadata("timestamp", serde_json::json!(chrono::Utc::now().to_rfc3339()))
```

### 5. Keep Output Concise

- Output is sent to the LLM, so keep it focused
- Use metadata for detailed statistics
- Truncate large outputs (with metadata indicating truncation)

## Migration Guide

If you have tools that manually construct `ToolResult`, update them as follows:

### Before (Manual Construction)

```rust
Ok(ToolResult {
    call_id: call.id.clone(),
    tool_name: self.name().to_string(),
    success: true,
    output: Some(format!("Processed {} items", count)),
    error: None,
    exit_code: Some(0),
    execution_time_ms: Some(duration),
    metadata: HashMap::new(),
})
```

### After (Standardized)

```rust
Ok(ToolResult::success(&call.id, self.name(),
                       format!("Processed {} items", count))
    .with_metadata("item_count", serde_json::json!(count))
    .with_execution_time(duration))
```

## Validation Checklist

When implementing a new tool or updating an existing one, ensure:

- [ ] Uses `ToolResult::success()` for successful operations
- [ ] Uses `ToolResult::error()` for failed operations
- [ ] Sets `call_id` from incoming `ToolCall.id`
- [ ] Uses `.with_metadata()` for structured data
- [ ] Uses `.with_execution_time()` for performance tracking
- [ ] Output is human-readable and concise
- [ ] Metadata contains machine-readable data
- [ ] Error messages are clear and actionable

## Reference

- `ToolResult` definition: `crates/sage-core/src/tools/types.rs`
- Example tools:
  - `ReadTool`: `crates/sage-tools/src/tools/file_ops/read.rs`
  - `BashTool`: `crates/sage-tools/src/tools/process/bash.rs`
  - `GlobTool`: `crates/sage-tools/src/tools/file_ops/glob.rs`

---

**Last Updated:** 2025-12-22 (MED-005)
