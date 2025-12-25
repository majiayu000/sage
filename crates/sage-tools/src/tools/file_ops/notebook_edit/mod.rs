//! Jupyter notebook editing tool

mod conversion;
mod operations;
mod schema;
mod types;
mod validation;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use std::path::PathBuf;

/// Tool for editing Jupyter notebook cells
pub struct NotebookEditTool {
    working_directory: PathBuf,
}

impl NotebookEditTool {
    /// Create a new notebook edit tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a notebook edit tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }
}

impl Default for NotebookEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "notebook_edit"
    }

    fn description(&self) -> &str {
        "Edit Jupyter notebook (.ipynb) cells. Supports three operations:
- replace: Replace the content of an existing cell (requires cell_id)
- insert: Insert a new cell at a position (requires cell_type; inserts after cell_id if provided, or at beginning)
- delete: Delete an existing cell (requires cell_id)

The notebook_path must be an absolute path to a .ipynb file.
Cell IDs can be found by reading the notebook file first."
    }

    fn schema(&self) -> ToolSchema {
        schema::create_schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let notebook_path = call.get_string("notebook_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'notebook_path' parameter".to_string())
        })?;

        let edit_mode = call
            .get_string("edit_mode")
            .unwrap_or_else(|| "replace".to_string());

        let mut result = match edit_mode.as_str() {
            "replace" => {
                let new_source = call.get_string("new_source").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'new_source' parameter for replace".to_string(),
                    )
                })?;
                let cell_id = call.get_string("cell_id");
                let cell_type = call.get_string("cell_type");
                operations::replace_cell(
                    self,
                    self.name(),
                    &notebook_path,
                    cell_id.as_deref(),
                    &new_source,
                    cell_type.as_deref(),
                )
                .await?
            }
            "insert" => {
                let new_source = call.get_string("new_source").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'new_source' parameter for insert".to_string(),
                    )
                })?;
                let cell_type = call.get_string("cell_type").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'cell_type' parameter for insert".to_string(),
                    )
                })?;
                let cell_id = call.get_string("cell_id");
                operations::insert_cell(
                    self,
                    self.name(),
                    &notebook_path,
                    cell_id.as_deref(),
                    &cell_type,
                    &new_source,
                )
                .await?
            }
            "delete" => {
                let cell_id = call.get_string("cell_id").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'cell_id' parameter for delete".to_string(),
                    )
                })?;
                operations::delete_cell(self, self.name(), &notebook_path, &cell_id).await?
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown edit_mode: {}. Use 'replace', 'insert', or 'delete'",
                    edit_mode
                )));
            }
        };

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        validation::validate_call(call)
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(60) // 1 minute
    }

    fn supports_parallel_execution(&self) -> bool {
        false // File operations should be sequential
    }
}

impl FileSystemTool for NotebookEditTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
