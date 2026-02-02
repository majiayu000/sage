---
name: write
description: File writing tool description
version: "1.1.0"
category: tool-description
variables:
  - READ_TOOL_NAME
  - EDIT_TOOL_NAME
---

Writes a file to the local filesystem (creates new file or completely overwrites existing).

## CRITICAL Rules

- ALWAYS prefer ${EDIT_TOOL_NAME} for modifying existing files
- NEVER write new files unless explicitly required
- NEVER proactively create documentation files (*.md, README) unless explicitly requested
- If overwriting an existing file, you MUST use ${READ_TOOL_NAME} first

## When to Use Write vs Edit

| Scenario | Tool |
|----------|------|
| Modify existing file | ${EDIT_TOOL_NAME} ✓ |
| Create new source file | Write (only if needed) |
| Create new config file | Write (only if needed) |
| Create documentation | NEVER (unless asked) |
| Overwrite entire file | Write (read first!) |

## Examples

<good-example>
user: "Create a new Python script for data processing"
assistant: [Uses Write tool to create data_processor.py]
</good-example>

<bad-example>
user: "Fix the bug in utils.py"
assistant: [Uses Write to overwrite entire utils.py]
This is WRONG - use ${EDIT_TOOL_NAME} for modifications!
</bad-example>

<bad-example>
user: "Add a new feature"
assistant: [Creates README.md documenting the feature]
This is WRONG - never create docs unless asked!
</bad-example>

<good-example>
user: "Please create a README for this project"
assistant: [Uses Write to create README.md]
This is CORRECT - user explicitly requested documentation
</good-example>

## Common Mistakes to Avoid

1. **Creating docs unprompted**: NEVER create .md files unless asked
2. **Overwriting when editing**: Use ${EDIT_TOOL_NAME} for modifications
3. **Not reading first**: Always read existing files before overwriting
4. **Adding emojis**: Only use emojis if user explicitly requests
