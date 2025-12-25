//! Schema definitions and Tool trait metadata implementations

use super::types::WriteTool;
use sage_core::tools::base::{FileSystemTool, Tool};
use sage_core::tools::types::{ToolParameter, ToolSchema};

impl Tool for WriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> &str {
        "Writes a file to the local filesystem.

Usage:
- This tool will overwrite the existing file if there is one at the provided path.
- If this is an existing file, you MUST use the Read tool first to read the file's contents. This tool will fail if you did not read the file first.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked.

Parameters:
- file_path (required): The absolute path to the file to write (must be absolute, not relative)
- content (required): The content to write to the file

Security:
- Parent directories will be created automatically if they don't exist
- Path validation ensures files are written within safe locations
- Existing files must be read first to prevent blind overwrites"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "file_path",
                    "The absolute path to the file to write (must be absolute, not relative)",
                ),
                ToolParameter::string("content", "The content to write to the file"),
            ],
        )
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(60) // 1 minute
    }

    fn supports_parallel_execution(&self) -> bool {
        false // File operations should be sequential
    }
}

impl FileSystemTool for WriteTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
