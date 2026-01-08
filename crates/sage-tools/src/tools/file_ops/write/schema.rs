//! Schema definitions and Tool trait implementations

use super::types::WriteTool;
use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use tracing::instrument;

#[async_trait]
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

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(60)) // 1 minute
    }

    fn supports_parallel_execution(&self) -> bool {
        false // File operations should be sequential
    }

    #[instrument(skip(self, call), fields(call_id = %call.id, file_path = call.get_string("file_path").as_deref().unwrap_or("<missing>")))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let content = call.get_string("content").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'content' parameter".to_string())
        })?;

        let mut result = self.write_file(&file_path, &content).await?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Check required parameters
        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        let _content = call.get_string("content").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'content' parameter".to_string())
        })?;

        // Validate that the path looks like an absolute path
        let path = std::path::Path::new(&file_path);
        if !path.is_absolute() && !file_path.starts_with('/') && !file_path.starts_with('C') {
            // Allow paths that start with / or drive letters
            // This is a soft warning - the tool may still work with relative paths
        }

        Ok(())
    }
}

impl FileSystemTool for WriteTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
