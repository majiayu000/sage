# Tool Response Builder Examples

This document provides practical examples of using the response builder utilities for creating standardized tool responses.

## Overview

The response builder utilities in `tools/utils/response_builder.rs` provide convenient helpers for creating standardized `ToolResult` responses. These utilities handle common patterns like timing, metadata, and formatting.

## Quick Reference

```rust
use crate::tools::utils::{
    FileOperationResponse,
    CommandResponse,
    NetworkResponse,
    SearchResponse,
    simple_success,
    simple_error,
    with_file_info,
    with_pagination,
};
```

## Example 1: File Operation Tool

### Before (Manual Construction)

```rust
use std::time::Instant;
use sage_core::tools::types::ToolResult;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let file_path = call.get_string("file_path")?;
    let start_time = Instant::now();

    // Read file
    let content = fs::read_to_string(&file_path).await?;
    let lines = content.lines().count();
    let bytes = content.len();

    // Manually build result
    let mut result = ToolResult::success(
        &call.id,
        self.name(),
        content.clone()
    );

    result.execution_time_ms = Some(start_time.elapsed().as_millis() as u64);
    result = result
        .with_metadata("file_path", file_path)
        .with_metadata("total_lines", lines)
        .with_metadata("bytes_processed", bytes);

    result.call_id = call.id.clone();
    Ok(result)
}
```

### After (Using Response Builder)

```rust
use crate::tools::utils::{FileOperationResponse, with_file_info};

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let file_path = call.get_string("file_path")?;

    // Read file
    let content = fs::read_to_string(&file_path).await?;
    let lines = content.lines().count();
    let bytes = content.len();

    // Use builder - cleaner and less error-prone
    let mut result = FileOperationResponse::new(&file_path, "Read")
        .with_file_metadata(
            &call.id,
            self.name(),
            content,
            vec![
                ("total_lines", serde_json::json!(lines)),
                ("bytes_read", serde_json::json!(bytes)),
            ],
        );

    result.call_id = call.id.clone();
    Ok(result)
}
```

## Example 2: Command Execution Tool

### Before (Manual Construction)

```rust
use std::time::Instant;
use tokio::process::Command;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let command = call.get_string("command")?;
    let start_time = Instant::now();

    let output = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .current_dir(&self.working_directory)
        .output()
        .await?;

    let execution_time = start_time.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut result = if output.status.success() {
        ToolResult::success(&call.id, self.name(), stdout.to_string())
    } else {
        ToolResult::error(&call.id, self.name(), stdout.to_string())
    };

    result.exit_code = output.status.code();
    result.execution_time_ms = Some(execution_time);
    result = result
        .with_metadata("command", command)
        .with_metadata("working_directory", self.working_directory.display().to_string());

    result.call_id = call.id.clone();
    Ok(result)
}
```

### After (Using Response Builder)

```rust
use crate::tools::utils::CommandResponse;
use tokio::process::Command;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let command = call.get_string("command")?;

    let output = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .current_dir(&self.working_directory)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Use builder - automatic timing and metadata
    let mut result = CommandResponse::new(&command)
        .with_working_directory(self.working_directory.display().to_string())
        .build(
            &call.id,
            self.name(),
            output.status.success(),
            stdout.to_string(),
            output.status.code(),
        );

    result.call_id = call.id.clone();
    Ok(result)
}
```

## Example 3: Network Request Tool

### Before (Manual Construction)

```rust
use std::time::Instant;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let url = call.get_string("url")?;
    let start_time = Instant::now();

    let response = reqwest::get(&url).await?;
    let status = response.status().as_u16();
    let body = response.text().await?;

    let execution_time = start_time.elapsed().as_millis() as u64;

    let mut result = ToolResult::success(&call.id, self.name(), body.clone());
    result.execution_time_ms = Some(execution_time);
    result = result
        .with_metadata("url", url)
        .with_metadata("status_code", status)
        .with_metadata("content_length", body.len());

    result.call_id = call.id.clone();
    Ok(result)
}
```

### After (Using Response Builder)

```rust
use crate::tools::utils::NetworkResponse;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let url = call.get_string("url")?;

    let response = reqwest::get(&url).await?;
    let status = response.status().as_u16();
    let body = response.text().await?;

    // Use builder - automatic timing and metadata
    let mut result = NetworkResponse::new(&url, "GET")
        .success(&call.id, self.name(), body, status);

    result.call_id = call.id.clone();
    Ok(result)
}
```

### With Error Handling

```rust
use crate::tools::utils::NetworkResponse;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let url = call.get_string("url")?;
    let builder = NetworkResponse::new(&url, "GET");

    let mut result = match reqwest::get(&url).await {
        Ok(response) => {
            let status = response.status().as_u16();
            match response.text().await {
                Ok(body) => builder.success(&call.id, self.name(), body, status),
                Err(e) => builder.error(&call.id, self.name(), format!("Failed to read response: {}", e)),
            }
        }
        Err(e) => builder.error(&call.id, self.name(), format!("Request failed: {}", e)),
    };

    result.call_id = call.id.clone();
    Ok(result)
}
```

## Example 4: Search Tool (Grep/Find)

### Before (Manual Construction)

```rust
use std::time::Instant;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let pattern = call.get_string("pattern")?;
    let search_path = call.get_string("path");
    let start_time = Instant::now();

    // Perform search
    let results = self.search_files(&pattern, search_path.as_deref())?;
    let total_matches = results.len();

    let execution_time = start_time.elapsed().as_millis() as u64;

    let output = if results.is_empty() {
        format!("No matches found for pattern: {}", pattern)
    } else {
        results.join("\n")
    };

    let mut result = ToolResult::success(&call.id, self.name(), output);
    result.execution_time_ms = Some(execution_time);
    result = result
        .with_metadata("pattern", pattern)
        .with_metadata("results_count", results.len())
        .with_metadata("total_matches", total_matches);

    if let Some(path) = search_path {
        result = result.with_metadata("search_path", path);
    }

    result.call_id = call.id.clone();
    Ok(result)
}
```

### After (Using Response Builder)

```rust
use crate::tools::utils::SearchResponse;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let pattern = call.get_string("pattern")?;
    let search_path = call.get_string("path");

    // Perform search
    let results = self.search_files(&pattern, search_path.as_deref())?;
    let total_matches = results.len();

    // Build response
    let mut builder = SearchResponse::new(&pattern);
    if let Some(path) = search_path {
        builder = builder.with_search_path(path);
    }

    let mut result = if results.is_empty() {
        builder.no_matches(&call.id, self.name())
    } else {
        builder.build(&call.id, self.name(), results, total_matches)
    };

    result.call_id = call.id.clone();
    Ok(result)
}
```

## Example 5: Simple Operations

### Using simple_success and simple_error

```rust
use crate::tools::utils::{simple_success, simple_error};
use std::time::Instant;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let start_time = Instant::now();

    // Simple validation or operation
    let value = call.get_string("value")?;

    let mut result = if value.len() > 100 {
        simple_error(
            &call.id,
            self.name(),
            "Value exceeds maximum length of 100 characters",
            start_time,
        )
    } else {
        simple_success(
            &call.id,
            self.name(),
            format!("Validated value: {}", value),
            start_time,
        )
    };

    result.call_id = call.id.clone();
    Ok(result)
}
```

## Example 6: Adding Common Metadata

### Using with_file_info

```rust
use crate::tools::utils::with_file_info;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let file_path = call.get_string("file_path")?;

    let content = fs::read_to_string(&file_path).await?;
    let lines = content.lines().count();
    let bytes = content.len();

    // Create basic result
    let result = ToolResult::success(&call.id, self.name(), content);

    // Add file metadata using helper
    let mut result = with_file_info(result, file_path, lines, bytes);

    result.call_id = call.id.clone();
    Ok(result)
}
```

### Using with_pagination

```rust
use crate::tools::utils::with_pagination;

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let offset = call.get_number("offset").unwrap_or(0.0) as usize;
    let limit = call.get_number("limit").unwrap_or(100.0) as usize;

    // Get data
    let all_items = self.fetch_all_items()?;
    let total = all_items.len();
    let page_items = all_items.into_iter().skip(offset).take(limit).collect::<Vec<_>>();

    let output = page_items.join("\n");

    // Create result with pagination metadata
    let result = ToolResult::success(&call.id, self.name(), output);
    let mut result = with_pagination(result, offset, limit, total);

    result.call_id = call.id.clone();
    Ok(result)
}
```

## Benefits

### 1. Consistency
All tools using these builders will have consistent response formats and metadata.

### 2. Less Boilerplate
Automatic handling of:
- Execution timing
- Standard metadata fields
- Error formatting

### 3. Type Safety
Builders enforce correct usage patterns at compile time.

### 4. Easier Testing
Builders can be easily mocked or tested independently.

### 5. Better Maintainability
Changes to response format can be made in one place (the builder) rather than across all tools.

## Migration Checklist

When migrating a tool to use response builders:

- [ ] Import appropriate builder(s) from `crate::tools::utils`
- [ ] Replace manual `ToolResult` construction with builder calls
- [ ] Remove manual timing code (builders handle this)
- [ ] Remove manual metadata setting (use builder methods)
- [ ] Ensure `call_id` is still set after builder usage
- [ ] Test the tool to verify output format is correct
- [ ] Update tests to use builders as well

## Best Practices

1. **Choose the Right Builder**: Use specialized builders (FileOperationResponse, CommandResponse, etc.) when they match your use case. Fall back to simple_success/simple_error for simple cases.

2. **Set call_id**: Always set `result.call_id = call.id.clone()` after using a builder.

3. **Add Custom Metadata**: Use `.with_metadata()` for tool-specific metadata beyond what the builder provides.

4. **Chain Methods**: Builders support method chaining for clean, readable code.

5. **Handle Errors Gracefully**: Use the error builders to provide clear, actionable error messages.

## See Also

- [TOOL_RESPONSE_STANDARD.md](./tools/TOOL_RESPONSE_STANDARD.md) - Complete standardization guide
- [sage-core/src/tools/types.rs](../../sage-core/src/tools/types.rs) - ToolResult type definition
- [tools/utils/response_builder.rs](./tools/utils/response_builder.rs) - Builder implementations
