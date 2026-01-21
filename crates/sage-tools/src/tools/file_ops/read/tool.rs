//! Read tool implementation

use super::reader;
use super::schema;
use super::types::MAX_LIMIT;
use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use std::path::PathBuf;

/// Tool for reading files with line numbers and pagination
pub struct ReadTool {
    working_directory: PathBuf,
}

impl ReadTool {
    /// Create a new read tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a read tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> &str {
        schema::description()
    }

    fn schema(&self) -> ToolSchema {
        schema::create_schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let file_path = call.require_string("file_path")?;

        let offset = call.get_number("offset").map(|n| n as usize);
        let limit = call.get_number("limit").map(|n| n as usize);

        let mut result = reader::read_file(self, self.name(), &file_path, offset, limit).await?;
        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        call.require_string("file_path")?;

        // Validate offset if provided
        if let Some(offset) = call.get_number("offset") {
            if offset < 0.0 {
                return Err(ToolError::InvalidArguments(
                    "Offset must be non-negative".to_string(),
                ));
            }
        }

        // Validate limit if provided
        if let Some(limit) = call.get_number("limit") {
            if limit <= 0.0 {
                return Err(ToolError::InvalidArguments(
                    "Limit must be greater than 0".to_string(),
                ));
            }
            if limit > MAX_LIMIT as f64 {
                return Err(ToolError::InvalidArguments(format!(
                    "Limit cannot exceed {} lines",
                    MAX_LIMIT
                )));
            }
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(30)) // 30 seconds
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Read operations can run in parallel
    }

    fn is_read_only(&self) -> bool {
        true // This tool only reads data
    }
}

impl FileSystemTool for ReadTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
