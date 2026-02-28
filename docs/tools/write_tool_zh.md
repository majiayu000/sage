# Write Tool

## Overview

The Write tool is a file system tool that allows creating new files or overwriting existing files with proper validation and security checks. It follows Claude Code's design pattern for the Write tool.

## Tool Information

- **Tool Name**: `Write`
- **Category**: File Operations
- **Location**: `/Users/Zhuanz/Desktop/code/Open/AI/code-agent/sage/crates/sage-tools/src/tools/file_ops/write.rs`

## Features

### Core Capabilities
- Create new files with specified content
- Overwrite existing files (with validation)
- Automatically create parent directories if they don't exist
- Support for absolute and relative paths

### Security Features
- Path validation to prevent writing to sensitive locations
- Working directory restrictions
- Read-before-write validation to prevent blind overwrites
- Safe path checking through `FileSystemTool` trait

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | Yes | The absolute path to the file to write (must be absolute, not relative) |
| `content` | string | Yes | The content to write to the file |

## Usage Examples

### Example 1: Create a New File

```rust
use sage_tools::WriteTool;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;

let tool = WriteTool::new();
let call = ToolCall::new("call-1", "Write", json!({
    "file_path": "/path/to/new/file.txt",
    "content": "Hello, World!"
}));

let result = tool.execute(&call).await?;
// Output: "Successfully created file: /path/to/new/file.txt (13 bytes)"
```

### Example 2: Create Nested Directories

```rust
let call = ToolCall::new("call-2", "Write", json!({
    "file_path": "/path/to/deeply/nested/file.txt",
    "content": "Content in nested directory"
}));

let result = tool.execute(&call).await?;
// Parent directories are created automatically
```

### Example 3: Overwrite After Reading

```rust
// First, mark the file as read (typically done by a Read tool)
let file_path = std::path::PathBuf::from("/path/to/existing.txt");
tool.mark_file_as_read(file_path);

// Now overwrite is allowed
let call = ToolCall::new("call-3", "Write", json!({
    "file_path": "/path/to/existing.txt",
    "content": "Updated content"
}));

let result = tool.execute(&call).await?;
// Output: "Successfully overwritten file: /path/to/existing.txt (15 bytes)"
```

### Example 4: Multiline Content

```rust
let content = r#"# README

This is a sample markdown file.

## Features
- Feature 1
- Feature 2

## Usage
Run with `cargo run`
"#;

let call = ToolCall::new("call-4", "Write", json!({
    "file_path": "/path/to/README.md",
    "content": content
}));

let result = tool.execute(&call).await?;
```

## Error Cases

### 1. Overwriting Without Reading

```rust
// This will fail if the file exists but hasn't been read
let call = ToolCall::new("call-5", "Write", json!({
    "file_path": "/path/to/existing.txt",
    "content": "New content"
}));

let result = tool.execute(&call).await;
// Error: "File exists but has not been read: /path/to/existing.txt.
//         You must use the Read tool first to examine the file before overwriting it."
```

### 2. Missing Parameters

```rust
// Missing 'content' parameter
let call = ToolCall::new("call-6", "Write", json!({
    "file_path": "/path/to/file.txt"
}));

let result = tool.execute(&call).await;
// Error: "Missing 'content' parameter"
```

### 3. Permission Denied

```rust
let call = ToolCall::new("call-7", "Write", json!({
    "file_path": "/etc/sensitive/system.conf",
    "content": "Malicious content"
}));

let result = tool.execute(&call).await;
// Error: "Access denied to path: /etc/sensitive/system.conf"
```

## Design Pattern

The Write tool follows Claude Code's design pattern:

1. **Read-Before-Write Validation**: Files that already exist must be read first before being overwritten. This prevents accidental data loss.

2. **Path Resolution**: Supports both absolute and relative paths, resolving relative paths against the working directory.

3. **Automatic Directory Creation**: Parent directories are created automatically, reducing the need for manual directory management.

4. **Security Checks**: Implements `FileSystemTool` trait for path validation and security checks.

## Integration

The Write tool is registered in the tool system:

```rust
// In sage-tools/src/tools/mod.rs
pub use file_ops::WriteTool;

pub fn get_default_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        // ...
        Arc::new(WriteTool::new()),
        // ...
    ]
}
```

## Testing

The Write tool includes comprehensive tests:

- ✅ `test_write_tool_create_new_file` - Creating new files
- ✅ `test_write_tool_with_subdirectories` - Creating nested directories
- ✅ `test_write_tool_overwrite_after_read` - Overwriting after reading
- ✅ `test_write_tool_overwrite_without_read_fails` - Validation of read-before-write
- ✅ `test_write_tool_missing_parameters` - Parameter validation
- ✅ `test_write_tool_empty_content` - Empty file creation
- ✅ `test_write_tool_multiline_content` - Multiline content handling
- ✅ `test_write_tool_binary_safe_content` - Special character handling
- ✅ `test_write_tool_schema` - Schema validation
- ✅ `test_write_tool_validation` - Argument validation

Run tests with:

```bash
cargo test --package sage-tools --lib file_ops::write
```

## Comparison with Edit Tool

| Feature | Write Tool | Edit Tool |
|---------|-----------|-----------|
| Purpose | Create/overwrite entire files | String replacement in files |
| Use Case | New files, complete rewrites | Targeted edits, incremental changes |
| Read Check | Required for overwrites | Required for edits |
| Directory Creation | Automatic | Automatic |
| Pattern | Claude Code Write | Custom Edit |

## Best Practices

1. **Always read before overwriting**: Use the Read tool to examine file contents before overwriting.

2. **Use absolute paths**: While relative paths work, absolute paths are preferred for clarity.

3. **Check result status**: Always check the `success` field in the result:
   ```rust
   let result = tool.execute(&call).await?;
   if result.success {
       println!("File written successfully");
   }
   ```

4. **Handle large content**: For very large files, consider streaming or chunking strategies.

5. **Use Edit for targeted changes**: If you only need to change part of a file, use the Edit tool instead.

## Implementation Details

### Key Components

1. **WriteTool Struct**:
   ```rust
   pub struct WriteTool {
       working_directory: PathBuf,
       read_files: Arc<Mutex<HashSet<PathBuf>>>,
   }
   ```

2. **Read Tracking**: Uses a thread-safe `HashSet` to track which files have been read.

3. **File System Integration**: Implements `FileSystemTool` trait for path resolution and safety checks.

4. **Execution Constraints**:
   - Max execution time: 60 seconds
   - Parallel execution: Not supported (sequential file operations)

### Success Messages

The tool returns different messages based on the operation:

- New file: `"Successfully created file: {path} ({bytes} bytes)"`
- Overwrite: `"Successfully overwritten file: {path} ({bytes} bytes)"`

## Future Enhancements

Potential improvements for future versions:

1. **Streaming Support**: For large files that don't fit in memory
2. **Atomic Writes**: Use temporary files and rename for atomic operations
3. **Backup Creation**: Automatically create backups before overwriting
4. **Content Validation**: Validate content format (e.g., JSON, YAML syntax)
5. **Permission Preservation**: Preserve file permissions when overwriting
6. **Dry-Run Mode**: Preview what would be written without actually writing

## Related Tools

- **ReadTool**: Read file contents
- **EditTool**: Perform string replacements in files
- **JsonEditTool**: Edit JSON files with structured operations
- **GlobTool**: Find files by pattern
- **GrepTool**: Search file contents

## References

- Claude Code Write Tool Specification
- FileSystemTool trait: `/Users/Zhuanz/Desktop/code/Open/AI/code-agent/sage/crates/sage-core/src/tools/base.rs`
- Tool types: `/Users/Zhuanz/Desktop/code/Open/AI/code-agent/sage/crates/sage-core/src/tools/types.rs`
