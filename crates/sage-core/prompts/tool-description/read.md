---
name: read
description: File reading tool description
version: "1.1.0"
category: tool-description
variables:
  - BASH_TOOL_NAME
  - EDIT_TOOL_NAME
---

Reads a file from the local filesystem. You can access any file directly by using this tool.
Assume this tool is able to read all files on the machine. If the User provides a path to a file assume that path is valid. It is okay to read a file that does not exist; an error will be returned.

## Usage Rules

- The file_path parameter must be an absolute path, not a relative path
- By default, it reads up to 2000 lines starting from the beginning of the file
- You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters
- Any lines longer than 2000 characters will be truncated
- Results are returned using cat -n format, with line numbers starting at 1
- This tool can only read files, not directories. To read a directory, use an ls command via the ${BASH_TOOL_NAME} tool.
- If you read a file that exists but has empty contents you will receive a system reminder warning in place of file contents.

## Supported File Types

- **Source code**: All programming languages (.py, .rs, .ts, .go, etc.)
- **Images**: PNG, JPG, GIF, etc. - contents are presented visually (multimodal)
- **PDFs**: Processed page by page, extracting text and visual content
- **Jupyter notebooks**: Returns all cells with outputs (code, text, visualizations)
- **Screenshots**: Works with all temporary file paths

## CRITICAL Rules

- ALWAYS read a file before editing it with ${EDIT_TOOL_NAME}
- NEVER guess file contents - always read first
- NEVER propose changes to code you haven't read
- You can call multiple tools in a single response - speculatively read multiple potentially useful files in parallel

## Examples

<good-example>
user: "Fix the bug in the login function"
assistant: [Uses Read tool to read src/auth/login.rs first, THEN proposes fix]
</good-example>

<bad-example>
user: "Fix the bug in the login function"
assistant: "The login function probably looks like this... here's how to fix it"
This is WRONG - never guess file contents!
</bad-example>

<good-example>
user: "What does the config file contain?"
assistant: [Reads /path/to/config.json and reports actual contents]
</good-example>

<bad-example>
user: "What does the config file contain?"
assistant: "A typical config file would contain..."
This is WRONG - always read the actual file!
</bad-example>
