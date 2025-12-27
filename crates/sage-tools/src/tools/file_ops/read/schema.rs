//! Schema definition for the Read tool

use sage_core::tools::types::{ToolParameter, ToolSchema};

/// Create the schema for the Read tool
pub fn create_schema() -> ToolSchema {
    ToolSchema::new(
        "Read",
        description(),
        vec![
            ToolParameter::string("file_path", "Absolute path to the file to read"),
            ToolParameter::number(
                "offset",
                "Line number to start reading from (0-indexed, default: 0)",
            )
            .optional(),
            ToolParameter::number("limit", "Maximum number of lines to read (default: 2000)")
                .optional(),
        ],
    )
}

/// Get the tool description
pub fn description() -> &'static str {
    r#"Reads a file from the local filesystem. You can access any file directly by using this tool.
Assume this tool is able to read all files on the machine. If the User provides a path to a file assume that path is valid. It is okay to read a file that does not exist; an error will be returned.

Usage:
- The file_path parameter must be an absolute path, not a relative path
- By default, it reads up to 2000 lines starting from the beginning of the file
- You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters
- Any lines longer than 2000 characters will be truncated
- Results are returned using cat -n format, with line numbers starting at 1
- This tool allows reading images (eg PNG, JPG, etc). When reading an image file the contents are presented visually as it is a multimodal LLM.
- This tool can read PDF files (.pdf). PDFs are processed page by page, extracting both text and visual content for analysis.
- This tool can read Jupyter notebooks (.ipynb files) and returns all cells with their outputs, combining code, text, and visualizations.
- This tool can only read files, not directories. To read a directory, use an ls command via the Bash tool.
- You can call multiple tools in a single response. It is always better to speculatively read multiple potentially useful files in parallel.
- If you read a file that exists but has empty contents you will receive a system reminder warning in place of file contents."#
}
