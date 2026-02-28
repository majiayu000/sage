# Grep Tool

The Grep tool is a powerful file content search tool that uses regex patterns to find matches across files and directories, similar to the popular `ripgrep` command-line tool.

## Overview

**Tool Name:** `Grep`

**Location:** `/Users/Zhuanz/Desktop/code/Open/AI/code-agent/sage/crates/sage-tools/src/tools/file_ops/grep.rs`

**Category:** File Operations

**Read-Only:** Yes (no side effects)

**Parallel Execution:** Supported

## Features

- **Regex Pattern Matching:** Full regex support with configurable flags
- **Multiple Output Modes:**
  - `content`: Show matching lines with content and optional context
  - `files_with_matches`: Show only file paths containing matches (default)
  - `count`: Show match counts per file
- **File Filtering:**
  - Glob patterns (e.g., `*.rs`, `**/*.ts`)
  - File type filtering (e.g., `rust`, `python`, `javascript`)
- **Context Lines:** Show lines before (-B), after (-A), or around (-C) matches
- **Case Sensitivity:** Optional case-insensitive search (-i flag)
- **Line Numbers:** Optional line number display (-n flag)
- **Multiline Matching:** Support for patterns that span multiple lines
- **Result Limiting:** Control output with `head_limit` and `offset` parameters
- **Smart Filtering:** Automatically skips:
  - Binary files
  - Common cache directories (node_modules, target, .git, etc.)
  - Hidden files

## Parameters

### Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `pattern` | string | The regex pattern to search for |

### Optional Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | string | current directory | File or directory to search in |
| `glob` | string | none | Filter files by glob pattern (e.g., '*.rs', '**/*.ts') |
| `type` | string | none | Filter by file type (see supported types below) |
| `output_mode` | string | `files_with_matches` | Output format: 'content', 'files_with_matches', or 'count' |
| `-i` | boolean | false | Case insensitive search |
| `-n` | boolean | true | Show line numbers (only for output_mode='content') |
| `-B` | number | 0 | Lines to show before each match |
| `-A` | number | 0 | Lines to show after each match |
| `-C` | number | 0 | Lines of context (before and after). Overrides -A and -B if set |
| `multiline` | boolean | false | Enable multiline mode where . matches newlines |
| `head_limit` | number | 0 | Limit output to first N results (0 = unlimited) |
| `offset` | number | 0 | Skip first N results |

### Supported File Types

The `type` parameter supports the following values:

- **Rust:** `rs`, `rust`
- **JavaScript:** `js`, `javascript` (includes .jsx, .mjs, .cjs)
- **TypeScript:** `ts`, `typescript` (includes .tsx)
- **Python:** `py`, `python`
- **Go:** `go`
- **Java:** `java`
- **C:** `c`
- **C++:** `cpp`, `c++` (includes .hpp, .h, .cc, .cxx)
- **Ruby:** `rb`, `ruby`
- **PHP:** `php`
- **HTML:** `html` (includes .htm)
- **CSS:** `css`
- **JSON:** `json`
- **YAML:** `yaml`, `yml`
- **XML:** `xml`
- **Markdown:** `md`, `markdown`
- **Text:** `txt`, `text`
- **TOML:** `toml`
- **SQL:** `sql`
- **Shell:** `sh`, `shell`, `bash` (includes .zsh)

## Usage Examples

### Example 1: Find Files with Pattern (Default Mode)

```rust
{
  "pattern": "TODO",
  "type": "rust"
}
```

Output:
```
src/main.rs
src/utils.rs
tests/integration.rs

Total: 3 file(s) with matches
```

### Example 2: Show Matching Content with Line Numbers

```rust
{
  "pattern": "async fn",
  "type": "rust",
  "output_mode": "content",
  "-n": true,
  "head_limit": 2
}
```

Output:
```
src/main.rs:
15:	async fn main() -> Result<()> {
42:	async fn process_data(data: &str) -> String {

src/lib.rs:
8:	async fn connect() -> Connection {
```

### Example 3: Search with Context Lines

```rust
{
  "pattern": "error",
  "output_mode": "content",
  "-A": 2,
  "-B": 1,
  "-n": true,
  "-i": true
}
```

Output shows 1 line before and 2 lines after each match.

### Example 4: Count Matches Per File

```rust
{
  "pattern": "function\\s+\\w+",
  "type": "javascript",
  "output_mode": "count"
}
```

Output:
```
src/app.js:12
src/utils.js:7
src/api.js:15

Total matches: 34
```

### Example 5: Glob Pattern Filtering

```rust
{
  "pattern": "version",
  "glob": "Cargo.toml"
}
```

Searches only in files named `Cargo.toml`.

### Example 6: Case-Insensitive Regex Search

```rust
{
  "pattern": "error|warning|panic",
  "-i": true,
  "type": "rust",
  "output_mode": "count"
}
```

### Example 7: Multiline Pattern Matching

```rust
{
  "pattern": "struct\\s+\\w+\\s*\\{[\\s\\S]*?\\}",
  "multiline": true,
  "type": "rust",
  "output_mode": "content"
}
```

Matches complete struct definitions across multiple lines.

### Example 8: Search Specific Directory

```rust
{
  "pattern": "import.*from",
  "path": "src/components",
  "type": "typescript",
  "output_mode": "files_with_matches"
}
```

## Output Modes

### 1. files_with_matches (Default)

Shows only the file paths that contain matches. This is the most compact output and is useful for:
- Getting a quick overview of affected files
- Piping to other tools
- When you only need to know which files contain the pattern

### 2. content

Shows the actual matching lines with optional:
- Line numbers (-n flag)
- Context lines (-A, -B, -C)
- Separator lines between context blocks

Best for:
- Reviewing the actual matches
- Understanding the context around matches
- Code review and analysis

### 3. count

Shows the number of matches per file in the format `filename:count`.

Best for:
- Statistics and metrics
- Finding files with the most occurrences
- Quick quantitative analysis

## Performance Considerations

1. **Large Directories:** The tool recursively walks directories. For very large codebases:
   - Use `type` or `glob` filters to reduce search scope
   - Use `head_limit` to limit results
   - Consider searching specific subdirectories with `path`

2. **Binary Files:** Automatically skipped to avoid errors and improve performance

3. **Ignored Directories:** Common build and cache directories are automatically skipped:
   - `node_modules`, `target`, `.git`, `.svn`, `.hg`
   - `dist`, `build`, `__pycache__`, `.pytest_cache`
   - `venv`, `.venv`

4. **Regex Complexity:** Complex regex patterns may be slower. Use simpler patterns when possible.

## Error Handling

The tool handles several error cases:

- **Invalid Regex:** Returns error with details about the regex problem
- **Path Not Found:** Returns error if the specified path doesn't exist
- **Permission Denied:** Returns error for paths outside allowed scope
- **Binary Files:** Silently skipped (no error)
- **Unreadable Files:** Silently skipped (no error)

## Comparison with Claude Code's Grep Tool

This implementation follows Claude Code's design pattern and provides equivalent functionality:

| Feature | Claude Code | Sage Implementation |
|---------|-------------|---------------------|
| Regex patterns | ✅ | ✅ |
| Output modes | ✅ (3 modes) | ✅ (3 modes) |
| Context lines | ✅ (-A, -B, -C) | ✅ (-A, -B, -C) |
| Case insensitive | ✅ (-i) | ✅ (-i) |
| Line numbers | ✅ (-n) | ✅ (-n) |
| File type filter | ✅ | ✅ (20+ types) |
| Glob filter | ✅ | ✅ |
| Multiline mode | ✅ | ✅ |
| Result limiting | ✅ | ✅ |
| Offset | ✅ | ✅ |

## Testing

The tool includes comprehensive tests covering:

- Basic pattern matching
- All output modes
- Case sensitivity
- Context lines (-A, -B, -C)
- Glob filtering
- Type filtering
- Head limit
- Invalid regex handling
- Empty results
- Schema validation

Run tests with:
```bash
cargo test --package sage-tools --lib tools::file_ops::grep::tests
```

## Example Usage in Code

See the complete example in `examples/grep_demo.rs`:

```bash
cargo run --package sage-tools --example grep_demo
```

## Integration

The Grep tool is automatically registered in the default tool set:

```rust
use sage_tools::tools::GrepTool;

// Create instance
let grep_tool = GrepTool::new();

// Or with custom working directory
let grep_tool = GrepTool::with_working_directory("/path/to/search");

// Execute search
let result = grep_tool.execute(&tool_call).await?;
```

## Best Practices

1. **Start Broad, Then Narrow:** Begin with `files_with_matches` to see which files contain matches, then use `content` mode to examine specific files.

2. **Use Type Filters:** Always use `type` parameter when searching in polyglot codebases to reduce noise.

3. **Leverage Context:** Use `-C 3` to see surrounding context when examining matches.

4. **Limit Results:** Use `head_limit` to avoid overwhelming output, especially in large codebases.

5. **Regex Efficiency:** Use specific patterns rather than overly broad ones for better performance.

6. **Case Sensitivity:** Remember that searches are case-sensitive by default. Use `-i` when needed.

## Future Enhancements

Potential improvements for future versions:

- [ ] Support for custom ignore patterns
- [ ] Parallel file scanning for better performance
- [ ] Support for fixed-string search (non-regex)
- [ ] File size limits
- [ ] Custom context separators
- [ ] Export results in different formats (JSON, CSV)
- [ ] Integration with syntax highlighting
- [ ] Support for searching compressed files

## License

Part of the Sage Agent project, licensed under MIT.
