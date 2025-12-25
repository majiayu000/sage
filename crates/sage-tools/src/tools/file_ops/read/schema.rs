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
    "Reads a file from the local filesystem with line numbers.

Features:
- Reads files with line numbers in format: '   1→content'
- Supports pagination with offset and limit parameters
- Default limit: 2000 lines
- Truncates lines longer than 2000 characters
- Detects and handles binary files (images, PDFs, executables)
- Provides metadata about the read operation

Usage:
- Read entire file (up to 2000 lines): {\"file_path\": \"/path/to/file.txt\"}
- Read with offset: {\"file_path\": \"/path/to/file.txt\", \"offset\": 100}
- Read with limit: {\"file_path\": \"/path/to/file.txt\", \"limit\": 50}
- Read specific range: {\"file_path\": \"/path/to/file.txt\", \"offset\": 100, \"limit\": 50}

Notes:
- file_path should be an absolute path
- offset is 0-indexed (offset: 0 starts at line 1)
- Line numbers in output are 1-indexed"
}
