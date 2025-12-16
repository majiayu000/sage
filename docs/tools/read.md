# Read Tool

The Read tool provides file reading capabilities with line numbers and pagination support, following Claude Code's design pattern.

## Overview

- **Tool Name**: `Read`
- **Category**: File Operations
- **Type**: Read-only
- **Parallel Execution**: Supported

## Features

1. **Line Numbering**: Reads files with line numbers in the format `   1→content`
2. **Pagination**: Supports `offset` and `limit` parameters for reading large files in chunks
3. **Line Truncation**: Automatically truncates lines longer than 2000 characters
4. **Binary Detection**: Detects and handles binary files (images, PDFs, executables)
5. **Metadata**: Provides detailed metadata about the read operation
6. **Security**: Validates paths and prevents directory traversal

## Usage

### Basic File Reading

Read entire file (up to 2000 lines by default):

```json
{
  "file_path": "/path/to/file.txt"
}
```

### Reading with Offset

Skip the first N lines:

```json
{
  "file_path": "/path/to/file.txt",
  "offset": 100
}
```

**Note**: `offset` is 0-indexed, so `offset: 0` starts at line 1, `offset: 100` starts at line 101.

### Reading with Limit

Limit the number of lines read:

```json
{
  "file_path": "/path/to/file.txt",
  "limit": 50
}
```

### Reading a Specific Range

Combine offset and limit for pagination:

```json
{
  "file_path": "/path/to/file.txt",
  "offset": 100,
  "limit": 50
}
```

This reads lines 101-150.

## Parameters

| Parameter   | Type   | Required | Default | Description |
|------------|--------|----------|---------|-------------|
| `file_path` | string | Yes      | -       | Absolute path to the file to read |
| `offset`    | number | No       | 0       | Line number to start reading from (0-indexed) |
| `limit`     | number | No       | 2000    | Maximum number of lines to read |

## Output Format

### Line Number Format

Lines are formatted with right-aligned line numbers followed by an arrow:

```
     1→First line content
     2→Second line content
    10→Tenth line content
   100→Hundredth line content
```

### Truncation Notice

When content is truncated (due to limit parameter), a notice is appended:

```
[Content truncated: showing lines 1-2000 of 5000 total lines. Use offset parameter to read more.]
```

### Long Line Truncation

Lines longer than 2000 characters are truncated with a notice:

```
  1234→Very long line content goes here... [line truncated, 3500 chars total]
```

## Metadata

Every read operation includes metadata:

```json
{
  "file_path": "/path/to/file.txt",
  "total_lines": 5000,
  "lines_read": 2000,
  "start_line": 1,
  "end_line": 2000,
  "truncated": true
}
```

## Binary File Detection

The tool automatically detects and handles binary files:

### Image Files

Supported extensions: `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.ico`, `.webp`, `.svg`

Output:
```
[Image file detected: /path/to/image.png]

This is a PNG image file. Binary content cannot be displayed as text.
File size: 12345 bytes
```

### PDF Files

Output:
```
[PDF file detected: /path/to/document.pdf]

This is a PDF file. Binary content cannot be displayed as text.
File size: 98765 bytes

To extract text from PDF, consider using a dedicated PDF processing tool.
```

### Other Binary Files

Extensions: `.exe`, `.dll`, `.so`, `.dylib`, `.bin`, `.zip`, `.tar`, `.gz`, `.rar`, `.7z`

Output:
```
[Binary file detected: /path/to/archive.zip]

This is a binary ZIP file. Content cannot be displayed as text.
File size: 54321 bytes
```

### Non-UTF8 Files

Files containing non-UTF8 data:
```
[Binary file detected: /path/to/file.bin]

File contains non-UTF8 data and cannot be displayed as text.
File size: 1024 bytes
```

## Error Handling

### File Not Found

```
Error: File not found: /path/to/nonexistent.txt
```

### Directory Instead of File

```
Error: Path is a directory, not a file: /path/to/directory
```

### Invalid Offset

```
Error: Offset 1000 exceeds total lines 500 in file
```

### File Too Large

Files larger than 100MB:
```
Error: File too large to read: 104857600 bytes. Use offset and limit parameters for large files.
```

## Validation Rules

1. **file_path**: Required, must be a valid string
2. **offset**: Optional, must be non-negative
3. **limit**: Optional, must be greater than 0 and less than or equal to 10000

## Examples

### Example 1: Reading a Configuration File

```json
{
  "file_path": "/home/user/config/app.json"
}
```

Output:
```
     1→{
     2→  "version": "1.0.0",
     3→  "database": {
     4→    "host": "localhost",
     5→    "port": 5432
     6→  }
     7→}
```

### Example 2: Reading a Large Log File in Chunks

First chunk:
```json
{
  "file_path": "/var/log/app.log",
  "limit": 1000
}
```

Next chunk:
```json
{
  "file_path": "/var/log/app.log",
  "offset": 1000,
  "limit": 1000
}
```

### Example 3: Viewing Specific Lines

To view lines 500-600:
```json
{
  "file_path": "/path/to/source.rs",
  "offset": 499,
  "limit": 101
}
```

## Best Practices

1. **Use Absolute Paths**: Always provide absolute file paths for consistency
2. **Paginate Large Files**: For files with thousands of lines, use offset and limit to read in chunks
3. **Check Metadata**: Use the `truncated` metadata flag to determine if there's more content
4. **Handle Binary Files**: Check the output for binary file detection messages
5. **Validate Paths**: Ensure file paths exist before reading to avoid errors

## Performance

- **Execution Time**: Default timeout of 30 seconds
- **Memory Usage**: Reads entire file into memory, then processes requested range
- **File Size Limit**: 100MB maximum file size
- **Parallel Execution**: Supports parallel reads (read-only operation)

## Security

1. **Path Validation**: Validates file paths to prevent directory traversal
2. **Size Limits**: Enforces maximum file size to prevent memory exhaustion
3. **Read-only**: Does not modify files or file system
4. **Working Directory**: Respects tool's working directory for relative paths

## Implementation Details

- **Language**: Rust
- **Async**: Fully asynchronous using Tokio
- **Location**: `/crates/sage-tools/src/tools/file_ops/read.rs`
- **Tests**: Comprehensive unit and integration tests included

## Related Tools

- **EditTool**: For modifying file contents
- **WriteTool**: For creating or overwriting files
- **GrepTool**: For searching within files
- **GlobTool**: For finding files by pattern

## Changelog

### Version 0.1.0 (Initial Release)
- Line-numbered file reading
- Pagination support with offset and limit
- Binary file detection (images, PDFs, executables)
- Line truncation for long lines
- Comprehensive metadata
- Security validations
- Empty file handling
