# Glob Tool

The Glob tool provides fast file pattern matching capabilities for finding files by name patterns in the Sage Agent system.

## Overview

- **Tool Name**: `Glob`
- **Category**: File Operations
- **Type**: Read-only
- **Parallel Execution**: Supported
- **Max Execution Time**: 30 seconds

## Features

- Fast pattern-based file searching
- Support for standard glob patterns
- Results sorted by modification time (newest first)
- Automatic result limiting (max 1000 files)
- Security-aware path validation
- Relative path output for cleaner results

## Supported Patterns

The Glob tool supports standard glob patterns:

| Pattern | Description | Example |
|---------|-------------|---------|
| `*` | Matches any sequence of characters (except `/`) | `*.rs` matches all Rust files |
| `**` | Matches any sequence including directories | `**/*.ts` matches all TypeScript files recursively |
| `?` | Matches any single character | `test?.rs` matches `test1.rs`, `test2.rs` |
| `[abc]` | Matches any character in the set | `[A-Z]*.md` matches markdown files starting with uppercase |
| `[a-z]` | Matches any character in the range | `file[0-9].txt` matches `file0.txt` through `file9.txt` |

**Note**: Brace expansion (e.g., `*.{js,ts}`) may not be supported on all systems. Use separate glob calls if you need to match multiple extensions.

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pattern` | string | Yes | Glob pattern to match files (e.g., `**/*.rs`) |
| `path` | string | No | Directory to search in (default: current working directory) |

## Examples

### Example 1: Find All Rust Files Recursively

```json
{
  "pattern": "**/*.rs"
}
```

Finds all `.rs` files in the current directory and all subdirectories.

### Example 2: Find Files in Specific Directory

```json
{
  "pattern": "**/*.ts",
  "path": "src"
}
```

Finds all TypeScript files within the `src` directory.

### Example 3: Find Configuration Files

```json
{
  "pattern": "**/Cargo.toml"
}
```

Finds all `Cargo.toml` files recursively.

### Example 4: Find Test Files

```json
{
  "pattern": "test_*.py"
}
```

Finds all Python test files in the current directory (non-recursive).

### Example 5: Character Class Pattern

```json
{
  "pattern": "[A-Z]*.md"
}
```

Finds all markdown files starting with an uppercase letter.

### Example 6: Single Character Wildcard

```json
{
  "pattern": "file?.txt"
}
```

Finds files like `file1.txt`, `fileA.txt`, but not `file10.txt`.

## Output Format

The tool returns:
- Number of files found
- List of matching file paths (sorted by modification time)
- Notification if results were limited to 1000 files
- Search directory information (if specified)

Example output:
```
Found 15 files matching pattern '**/*.rs':

1. src/main.rs
2. src/lib.rs
3. src/agent/mod.rs
4. src/tools/base.rs
...
```

## Error Handling

The tool will return errors for:
- Missing `pattern` parameter
- Invalid glob pattern syntax
- Non-existent search directory
- Path is not a directory
- Permission denied for path access

## Security

The Glob tool implements several security measures:
- Path validation to prevent access outside allowed directories
- Skips files that fail security checks
- Limits result count to prevent resource exhaustion
- Read-only operation (no file modifications)

## Performance Characteristics

- **Speed**: Fast pattern matching using the `glob` crate
- **Memory**: Efficient handling of large result sets
- **Scalability**: Results limited to 1000 files to prevent memory issues
- **Sorting**: Results sorted by modification time for relevance

## Use Cases

1. **Code Search**: Find all source files of a specific type
2. **Project Analysis**: Locate configuration files across a project
3. **Test Discovery**: Find all test files matching a pattern
4. **Build Tools**: Locate files for processing or compilation
5. **Documentation**: Find all documentation files

## Comparison with Other Tools

| Feature | Glob | Read | CodebaseRetrieval |
|---------|------|------|-------------------|
| Pattern Matching | ✅ File names | ❌ | ✅ Content |
| Recursive Search | ✅ | ❌ | ✅ |
| Content Search | ❌ | ✅ | ✅ |
| Sorted Results | ✅ By time | ❌ | ✅ By relevance |
| Speed | Fast | N/A | Moderate |

## Implementation Details

- **Language**: Rust
- **Dependencies**: `glob` crate (v0.3)
- **Module**: `sage-tools::file_ops::glob`
- **Struct**: `GlobTool`
- **Trait**: Implements `Tool` and `FileSystemTool`

## Testing

The Glob tool includes comprehensive tests covering:
- Basic pattern matching
- Recursive patterns
- Character classes
- Single character wildcards
- Search path specification
- Error conditions
- Edge cases

Run tests with:
```bash
cargo test --package sage-tools glob_tool
```

## Integration

### Using in Rust Code

```rust
use sage_tools::GlobTool;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;

let glob_tool = GlobTool::new();

let call = ToolCall::new(
    "call-1",
    "Glob",
    [("pattern".to_string(), json!("**/*.rs"))]
        .into_iter()
        .collect()
);

let result = glob_tool.execute(&call).await?;
```

### Registering with Agent

The Glob tool is automatically registered when using `get_default_tools()`:

```rust
use sage_tools::get_default_tools;

let tools = get_default_tools();
// Glob tool is included
```

## Best Practices

1. **Use Specific Patterns**: More specific patterns return faster results
2. **Limit Search Scope**: Use the `path` parameter to search specific directories
3. **Handle Large Results**: Be aware of the 1000 file limit
4. **Pattern Testing**: Test patterns with small directories first
5. **Error Handling**: Always check for errors, especially with user-provided patterns

## Limitations

1. **Brace Expansion**: `{a,b,c}` syntax may not work on all systems
2. **Result Limit**: Maximum 1000 files returned
3. **File Metadata**: Only uses modification time for sorting
4. **Pattern Complexity**: Very complex patterns may be slower
5. **Hidden Files**: Behavior with hidden files depends on the glob pattern

## Future Enhancements

Potential improvements for future versions:
- Configurable result limit
- Additional sorting options (size, name, extension)
- File type filtering
- Exclude patterns
- Parallel directory traversal
- Progress reporting for large searches

## Related Tools

- **Read Tool**: For reading file contents
- **CodebaseRetrieval Tool**: For content-based searching
- **Bash Tool**: For advanced file operations using shell commands

## References

- [Rust glob crate documentation](https://docs.rs/glob/)
- [Glob pattern syntax](https://en.wikipedia.org/wiki/Glob_(programming))
- [Sage Agent documentation](../../README.md)
