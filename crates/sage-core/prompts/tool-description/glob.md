---
name: glob
description: File pattern matching tool description
version: "1.1.0"
category: tool-description
variables:
  - TASK_TOOL_NAME
  - BASH_TOOL_NAME
---

Fast file pattern matching tool that works with any codebase size.

## Capabilities

- Supports glob patterns like `**/*.js` or `src/**/*.ts`
- Returns matching file paths sorted by modification time
- Use this tool when you need to find files by name patterns

## Pattern Syntax

| Pattern | Matches |
|---------|---------|
| `*.rs` | All .rs files in current directory |
| `**/*.rs` | All .rs files recursively |
| `src/**/*.ts` | All .ts files under src/ |
| `**/test_*.py` | All Python test files |
| `{src,lib}/**/*.rs` | .rs files in src/ or lib/ |

## When to Use

<good-example>
"Find all Rust files" → Glob with pattern "**/*.rs"
</good-example>

<good-example>
"Find the config file" → Glob with pattern "**/config.{json,yaml,toml}"
</good-example>

<bad-example>
Using ${BASH_TOOL_NAME} with `find . -name "*.rs"`
This is WRONG - use Glob tool instead!
</bad-example>

## When NOT to Use

- For open-ended searches requiring multiple rounds → use ${TASK_TOOL_NAME} instead
- For searching file contents → use Grep tool instead
- For complex directory traversal → use ${TASK_TOOL_NAME} with Explore agent

## Performance Tips

- You can call multiple Glob tools in parallel for different patterns
- Speculatively search multiple patterns if you're unsure which one will match
- More specific patterns (e.g., `src/**/*.rs`) are faster than broad ones (e.g., `**/*`)
