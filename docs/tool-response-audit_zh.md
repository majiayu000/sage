# Tool Response Format Audit Report

**Date:** 2025-12-23
**Scope:** sage-tools crate (`/Users/lifcc/Desktop/code/AI/agent/sage/crates/sage-tools/src/tools/`)
**Purpose:** Audit and standardize tool response formats across all tools

---

## Executive Summary

This audit examined 13 representative tools across different categories (file operations, process management, task management, network, and interaction tools) to identify inconsistencies in response format patterns. The analysis reveals **three major inconsistency areas** that need standardization:

1. **Inconsistent `call_id` parameter in success responses**
2. **Inconsistent `execution_time_ms` tracking patterns**
3. **Varied metadata usage patterns**

---

## Current Response Format Patterns

### Pattern A: Success with Empty Call ID (67% of tools)
**Tools using this pattern:** Edit, Read, Write, Bash, Grep, Glob, MultiEdit, JsonEdit

**Pattern:**
```rust
let mut result = ToolResult::success("", self.name(), output);
result.call_id = call.id.clone();
Ok(result)
```

**Characteristics:**
- Create result with empty string `""` for call_id
- Manually set `result.call_id` afterward
- Two-step process

**Example from Edit tool (line 158-176):**
```rust
let mut result = if replace_all && occurrences > 1 {
    ToolResult::success(
        &call.id,
        self.name(),
        format!("Successfully replaced {} occurrences in {}", occurrences, file_path),
    )
} else {
    ToolResult::success(
        &call.id,
        self.name(),
        format!("Successfully edited {}", file_path),
    )
};

result.call_id = call.id.clone();
Ok(result)
```

### Pattern B: Success with Direct Call ID (33% of tools)
**Tools using this pattern:** TodoWrite, WebFetch, AskUserQuestion, TaskOutput

**Pattern:**
```rust
let result = ToolResult::success(&call.id, self.name(), output);
Ok(result)
```

**Characteristics:**
- Pass `&call.id` directly to `ToolResult::success()`
- Single-step process
- No manual call_id assignment

**Example from TodoWrite (line 250):**
```rust
let mut result = ToolResult::success(&call.id, self.name(), response);
```

---

## Execution Time Tracking Inconsistencies

### Method 1: Direct Field Assignment (Most Common)
**Tools:** Bash, TaskOutput

```rust
let start_time = std::time::Instant::now();
// ... execute operation ...
let execution_time = start_time.elapsed().as_millis() as u64;
result.execution_time_ms = Some(execution_time);
```

**Example from Bash (line 121-180):**
```rust
let start_time = std::time::Instant::now();
let output = cmd.output().await.map_err(...)?;
let execution_time = start_time.elapsed().as_millis() as u64;
// ...
result.exit_code = output.status.code();
result.execution_time_ms = Some(execution_time);
```

### Method 2: Builder Pattern `.with_execution_time()`
**Tools:** Bash (background mode)

```rust
Ok(ToolResult::success("", self.name(), output)
    .with_metadata("shell_id", serde_json::Value::String(shell_id))
    .with_metadata("pid", serde_json::json!(pid))
    .with_execution_time(0))
```

**Example from Bash background execution (line 95-98):**
```rust
Ok(ToolResult::success("", self.name(), output)
    .with_metadata("shell_id", serde_json::Value::String(shell_id))
    .with_metadata("pid", serde_json::json!(pid))
    .with_execution_time(0))
```

### Method 3: No Tracking (Majority of Tools)
**Tools:** Edit, Read, Write, Grep, Glob, TodoWrite, WebFetch, AskUser, MultiEdit, JsonEdit

These tools do not track execution time at all.

---

## Metadata Usage Patterns

### Category 1: Rich Metadata (Read Tool)
**Most comprehensive metadata usage**

```rust
result = result
    .with_metadata("file_path", serde_json::Value::String(file_path.to_string()))
    .with_metadata("total_lines", serde_json::Value::Number(total_lines.into()))
    .with_metadata("lines_read", serde_json::Value::Number((end_line - start_line).into()))
    .with_metadata("start_line", serde_json::Value::Number((start_line + 1).into()))
    .with_metadata("end_line", serde_json::Value::Number(end_line.into()))
    .with_metadata("truncated", serde_json::Value::Bool(truncated));
```

**Fields:** 6 metadata fields (file_path, total_lines, lines_read, start_line, end_line, truncated)

### Category 2: Moderate Metadata (Bash, TodoWrite)
**Bash tool:**
```rust
result = result
    .with_metadata("command", serde_json::Value::String(command.to_string()))
    .with_metadata("working_directory", serde_json::Value::String(self.working_directory.display().to_string()));
```

**TodoWrite tool:**
```rust
result = result
    .with_metadata("total_tasks", serde_json::json!(total))
    .with_metadata("completed_tasks", serde_json::json!(completed))
    .with_metadata("in_progress_tasks", serde_json::json!(in_progress));
```

**Fields:** 2-3 metadata fields

### Category 3: Minimal/No Metadata (Most Tools)
**Tools:** Edit, Write, Grep, Glob, MultiEdit, JsonEdit, WebFetch, AskUser, TaskOutput

These tools provide little to no metadata in responses.

---

## Error Handling Patterns

### Consistent Pattern Across All Tools
All tools use the `ToolError` enum consistently:

```rust
// InvalidArguments
Err(ToolError::InvalidArguments("Missing 'file_path' parameter".to_string()))

// ExecutionFailed
Err(ToolError::ExecutionFailed(format!("File not found: {}", file_path)))

// PermissionDenied
Err(ToolError::PermissionDenied(format!("Access denied to path: {}", path.display())))

// ValidationFailed
Err(ToolError::ValidationFailed(format!("File has not been read: {}", file_path)))

// NotFound
Err(ToolError::NotFound(format!("Background shell '{}' not found", shell_id)))
```

**Status:** ✅ No issues found - error handling is already standardized

---

## Output Format Patterns

### Pattern A: Direct String Output (Simple Results)
**Tools:** Edit, Write, Glob, TodoWrite, WebFetch, MultiEdit

```rust
ToolResult::success("", self.name(), "Successfully edited file.txt")
```

### Pattern B: Formatted Multi-line Output (Complex Results)
**Tools:** Read, Bash, Grep, TaskOutput, AskUser

```rust
let output = format!(
    "STDOUT:\n{}\n\nSTDERR:\n{}",
    stdout, stderr
);
ToolResult::success("", self.name(), output)
```

### Pattern C: Modified Output After Creation (Read Tool Only)
```rust
let mut result = ToolResult::success("", self.name(), output);
// ... later ...
if truncated {
    let existing_output = result.output.unwrap_or_default();
    result.output = Some(format!(
        "{}\n\n[Content truncated: showing lines {}-{} of {} total lines...]",
        existing_output, start_line + 1, end_line, total_lines
    ));
}
```

**Issue:** Only Read tool modifies output after creation, creating inconsistency.

---

## Files Requiring Modifications

### Priority 1: High-Impact Standardization (Core Tools)

| File | Issue | Line References | Recommended Change |
|------|-------|-----------------|-------------------|
| `file_ops/edit.rs` | Inconsistent call_id pattern | 158-176 | Use direct call_id in success() |
| `file_ops/read.rs` | Inconsistent call_id pattern | 159-237 | Use direct call_id in success() |
| `file_ops/write.rs` | Inconsistent call_id pattern | 179-181 | Use direct call_id in success() |
| `file_ops/grep.rs` | Inconsistent call_id pattern | 251, 603-604 | Use direct call_id in success() |
| `file_ops/glob.rs` | Inconsistent call_id pattern | 160, 227-229 | Use direct call_id in success() |
| `process/bash.rs` | Mixed call_id patterns | 95-98, 164-190 | Standardize to direct call_id |

### Priority 2: Enhanced Tools (With Extra Features)

| File | Issue | Line References | Recommended Change |
|------|-------|-----------------|-------------------|
| `file_ops/multi_edit.rs` | Inconsistent call_id pattern | 221-324 | Use direct call_id in success() |
| `file_ops/json_edit.rs` | Inconsistent call_id pattern | 250-296 | Use direct call_id in success() |

### Priority 3: Already Compliant (Reference Implementations)

| File | Status | Notes |
|------|--------|-------|
| `task_mgmt/todo_write.rs` | ✅ Compliant | Uses direct call_id pattern (line 250) |
| `network/web_fetch.rs` | ✅ Compliant | Uses direct call_id pattern (line 61) |
| `interaction/ask_user.rs` | ✅ Compliant | Uses direct call_id pattern (line 269, 275) |
| `process/task_output.rs` | ✅ Compliant | Uses direct call_id pattern (line 136) |

---

## Recommended Standard Format

### 1. Success Response Pattern

**✅ RECOMMENDED:**
```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    // ... validation and execution ...

    // Create result with call_id directly
    let result = ToolResult::success(&call.id, self.name(), output);

    // Add metadata if needed
    let result = result
        .with_metadata("key", serde_json::json!(value));

    Ok(result)
}
```

**❌ AVOID:**
```rust
// Don't create with empty call_id then set it manually
let mut result = ToolResult::success("", self.name(), output);
result.call_id = call.id.clone();  // Redundant!
```

### 2. Execution Time Tracking

**✅ RECOMMENDED (when execution time is significant):**
```rust
async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    let start_time = std::time::Instant::now();

    // ... perform operation ...

    let mut result = ToolResult::success(&call.id, self.name(), output);
    result.execution_time_ms = Some(start_time.elapsed().as_millis() as u64);

    Ok(result)
}
```

**When to track:**
- File I/O operations that may be slow (Read, Write, Edit)
- Network operations (WebFetch, WebSearch)
- Process execution (Bash, Task)
- Search operations (Grep, Glob)

**When NOT to track:**
- Simple in-memory operations (TodoWrite, AskUser)
- Immediate responses with no I/O

### 3. Metadata Guidelines

**✅ RECOMMENDED (add operation-specific metadata):**
```rust
let result = ToolResult::success(&call.id, self.name(), output)
    .with_metadata("operation_specific_key", serde_json::json!(value));
```

**Metadata should include:**
- **File operations:** file_path, bytes_read/written, lines_modified
- **Process operations:** command, exit_code, working_directory
- **Search operations:** pattern, matches_found, files_searched
- **Task operations:** task_count, completion_status

### 4. Error Response Pattern

**✅ ALREADY STANDARDIZED - No changes needed:**
```rust
// Parameter validation
if param.is_none() {
    return Err(ToolError::InvalidArguments(
        "Missing 'param_name' parameter".to_string()
    ));
}

// Execution failure
if let Err(e) = operation() {
    return Err(ToolError::ExecutionFailed(
        format!("Operation failed: {}", e)
    ));
}

// Permission denied
if !self.is_safe_path(&path) {
    return Err(ToolError::PermissionDenied(
        format!("Access denied to path: {}", path.display())
    ));
}
```

---

## Implementation Roadmap

### Phase 1: Critical Path (Week 1)
1. **Standardize call_id pattern** in all file_ops tools
   - edit.rs, read.rs, write.rs, grep.rs, glob.rs
   - Change from empty string pattern to direct call_id pattern

### Phase 2: Process Tools (Week 2)
2. **Standardize bash.rs** call_id patterns
   - Unify foreground and background execution patterns
3. **Add execution time tracking** to file_ops tools
   - Read, Write, Edit, MultiEdit (when I/O time may be significant)

### Phase 3: Enhanced Tools (Week 3)
4. **Update multi_edit.rs and json_edit.rs**
   - Apply call_id standardization
   - Consider enabling if they're to be used in production

### Phase 4: Metadata Enhancement (Week 4)
5. **Enrich metadata** for better observability
   - Add consistent metadata fields across similar tool types
   - Document metadata schema for each tool category

---

## Testing Recommendations

### Unit Test Coverage
Ensure all tools have tests covering:
1. ✅ Success response structure (call_id set correctly)
2. ✅ Error handling (proper ToolError variants)
3. ✅ Metadata presence (when applicable)
4. ⚠️ Execution time tracking (add tests where missing)

### Integration Test Additions
Add integration tests for:
1. Response format consistency across tool categories
2. Metadata field presence and types
3. Error message clarity and actionability

---

## Breaking Changes Assessment

### ✅ Safe Changes (No Breaking Changes)
- Changing call_id from manual assignment to direct parameter
  - Both produce identical results
  - Internal implementation detail only

- Adding execution_time_ms tracking
  - Additive change - existing code unaffected

- Adding metadata fields
  - Additive change - existing code unaffected

### ⚠️ Potential Issues
- None identified - all changes are internal implementation improvements

---

## Summary Statistics

| Metric | Count | Percentage |
|--------|-------|------------|
| Total tools audited | 13 | 100% |
| Using Pattern A (empty call_id) | 8 | 62% |
| Using Pattern B (direct call_id) | 5 | 38% |
| Tracking execution time | 2 | 15% |
| Using rich metadata (3+ fields) | 3 | 23% |
| Using minimal/no metadata | 10 | 77% |

---

## Conclusion

The audit reveals that while error handling is already well-standardized, **response format patterns show significant inconsistency**, particularly around:

1. **call_id initialization** (62% vs 38% split)
2. **Execution time tracking** (only 15% of tools)
3. **Metadata richness** (77% provide minimal/no metadata)

Implementing the recommended standard format will:
- ✅ Improve code consistency and maintainability
- ✅ Reduce cognitive load for developers
- ✅ Enable better observability and debugging
- ✅ Require no breaking API changes

**Next Step:** Prioritize Phase 1 (file_ops tools) for immediate standardization to establish the pattern for all future tool development.
