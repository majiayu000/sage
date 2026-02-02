---
name: grep
description: Content search tool description
version: "1.1.0"
category: tool-description
variables:
  - GREP_TOOL_NAME
  - BASH_TOOL_NAME
  - TASK_TOOL_NAME
---

A powerful search tool built on ripgrep for searching file contents.

## CRITICAL Rules

- ALWAYS use ${GREP_TOOL_NAME} for search tasks
- NEVER invoke `grep` or `rg` as a ${BASH_TOOL_NAME} command
- The ${GREP_TOOL_NAME} tool has been optimized for correct permissions and access

## Capabilities

- Supports full regex syntax (e.g., `log.*Error`, `function\s+\w+`)
- Filter files with glob parameter (e.g., `*.js`, `**/*.tsx`)
- Filter by file type (e.g., `js`, `py`, `rust`)
- Output modes: `content`, `files_with_matches` (default), `count`

## Pattern Syntax (ripgrep)

| Pattern | Matches |
|---------|---------|
| `TODO` | Literal "TODO" |
| `log.*Error` | "log" followed by "Error" |
| `fn\s+\w+` | Function definitions |
| `interface\{\}` | Literal `interface{}` (escape braces) |

## Examples

<good-example>
Find all TODO comments:
pattern: "TODO|FIXME|HACK"
glob: "**/*.rs"
</good-example>

<good-example>
Find function definitions:
pattern: "fn\s+process"
type: "rust"
output_mode: "content"
</good-example>

<bad-example>
Using ${BASH_TOOL_NAME} with `grep -r "TODO" .`
This is WRONG - use ${GREP_TOOL_NAME} tool instead!
</bad-example>

<bad-example>
Using ${BASH_TOOL_NAME} with `rg "pattern" --type rust`
This is WRONG - use ${GREP_TOOL_NAME} tool instead!
</bad-example>

## Multiline Matching

By default, patterns match within single lines only.
For cross-line patterns, use `multiline: true`:

<example>
Find struct with specific field:
pattern: "struct\s+Config\s*\{[\s\S]*?timeout"
multiline: true
</example>

## When to Use ${TASK_TOOL_NAME} Instead

- Open-ended searches requiring multiple rounds
- Complex codebase exploration
- When you need to understand context around matches
