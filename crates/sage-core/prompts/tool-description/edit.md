---
name: edit
description: File editing tool description
version: "1.1.0"
category: tool-description
variables:
  - READ_TOOL_NAME
---

Performs exact string replacements in files.

## CRITICAL Rules

- You MUST use ${READ_TOOL_NAME} tool at least once before editing. This tool will error if you attempt an edit without reading the file first.
- ALWAYS prefer editing existing files. NEVER create new files unless explicitly required.
- NEVER include line numbers in old_string or new_string - only the actual content.
- The edit will FAIL if `old_string` is not unique. Provide more surrounding context to make it unique.

## Usage Guidelines

- When editing text from ${READ_TOOL_NAME} output, preserve exact indentation (tabs/spaces)
- The line number prefix format is: spaces + line number + tab. Everything AFTER that tab is the actual file content.
- Only use emojis if user explicitly requests it.
- Use `replace_all` for renaming variables or replacing all instances of a string.

## Examples

<good-example>
Reading file shows:
    42→    fn process_data(input: &str) -> Result<Data> {
    43→        let parsed = parse(input)?;

To change function name:
old_string: "fn process_data(input: &str)"
new_string: "fn transform_data(input: &str)"
</good-example>

<bad-example>
old_string: "42→    fn process_data"
This is WRONG - never include line numbers!
</bad-example>

<good-example>
For non-unique string, add more context:
old_string: "let result = process();\n        println!(\"Done\");"
new_string: "let result = process();\n        info!(\"Processing complete\");"
</good-example>

<bad-example>
old_string: "let result"
This is WRONG if "let result" appears multiple times - add more context!
</bad-example>

<good-example>
To rename all occurrences of a variable:
old_string: "user_name"
new_string: "username"
replace_all: true
</good-example>

## Common Mistakes to Avoid

1. **Editing without reading**: ALWAYS read first
2. **Including line numbers**: Only use actual content
3. **Insufficient context**: Add surrounding lines if string isn't unique
4. **Wrong indentation**: Match exact whitespace from the file
5. **Creating new files**: Edit existing files unless new file is explicitly needed
