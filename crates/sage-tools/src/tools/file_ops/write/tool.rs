//! Tool trait execute and validate implementations

use super::types::WriteTool;
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult};
use tracing::instrument;

#[async_trait]
impl Tool for WriteTool {
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
