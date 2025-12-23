//! Jupyter notebook editing tool

use async_trait::async_trait;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Jupyter notebook cell representation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotebookCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    cell_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_count: Option<serde_json::Value>,
    metadata: serde_json::Value,
    source: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    outputs: Option<Vec<serde_json::Value>>,
}

/// Jupyter notebook structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Notebook {
    cells: Vec<NotebookCell>,
    metadata: serde_json::Value,
    nbformat: u32,
    nbformat_minor: u32,
}

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

    /// Parse source field to string
    #[allow(dead_code)]
    fn source_to_string(source: &serde_json::Value) -> String {
        match source {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }

    /// Convert string to source field (array of strings with newlines preserved)
    fn string_to_source(content: &str) -> serde_json::Value {
        if content.is_empty() {
            return serde_json::Value::Array(vec![]);
        }

        let lines: Vec<serde_json::Value> = content
            .split('\n')
            .enumerate()
            .map(|(i, line)| {
                // Add newline to all lines except the last one
                if i < content.split('\n').count() - 1 {
                    serde_json::Value::String(format!("{}\n", line))
                } else {
                    serde_json::Value::String(line.to_string())
                }
            })
            .collect();

        serde_json::Value::Array(lines)
    }

    /// Replace cell content
    async fn replace_cell(
        &self,
        notebook_path: &str,
        cell_id: Option<&str>,
        new_source: &str,
        cell_type: Option<&str>,
    ) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(notebook_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Read and parse notebook
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read notebook file '{}': {}",
                notebook_path, e
            ))
        })?;

        let mut notebook: Notebook =
            serde_json::from_str(&content).map_err(|e| ToolError::Json(e))?;

        // Find cell to replace
        let cell_index = if let Some(id) = cell_id {
            notebook
                .cells
                .iter()
                .position(|c| c.id.as_deref() == Some(id))
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(format!("Cell with id '{}' not found", id))
                })?
        } else {
            return Err(ToolError::InvalidArguments(
                "cell_id is required for replace operation".to_string(),
            ));
        };

        // Update cell
        let cell = &mut notebook.cells[cell_index];
        cell.source = Self::string_to_source(new_source);

        // Update cell type if provided
        if let Some(ct) = cell_type {
            cell.cell_type = ct.to_string();
            // Clear outputs if changing to markdown
            if ct == "markdown" {
                cell.outputs = None;
                cell.execution_count = None;
            }
        }

        // Write back to file
        let new_content =
            serde_json::to_string_pretty(&notebook).map_err(|e| ToolError::Json(e))?;

        fs::write(&path, new_content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to write updated notebook to '{}': {}",
                notebook_path, e
            ))
        })?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully replaced content of cell '{}' in {}",
                cell_id.unwrap_or("unknown"),
                notebook_path
            ),
        ))
    }

    /// Insert new cell
    async fn insert_cell(
        &self,
        notebook_path: &str,
        cell_id: Option<&str>,
        cell_type: &str,
        new_source: &str,
    ) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(notebook_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Read and parse notebook
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read notebook file for insertion '{}': {}",
                notebook_path, e
            ))
        })?;

        let mut notebook: Notebook =
            serde_json::from_str(&content).map_err(|e| ToolError::Json(e))?;

        // Create new cell
        let new_cell = NotebookCell {
            id: Some(uuid::Uuid::new_v4().to_string()),
            cell_type: cell_type.to_string(),
            execution_count: if cell_type == "code" {
                Some(serde_json::Value::Null)
            } else {
                None
            },
            metadata: serde_json::json!({}),
            source: Self::string_to_source(new_source),
            outputs: if cell_type == "code" {
                Some(vec![])
            } else {
                None
            },
        };

        // Find insertion position
        let insert_pos = if let Some(id) = cell_id {
            // Insert after the specified cell
            let pos = notebook
                .cells
                .iter()
                .position(|c| c.id.as_deref() == Some(id))
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(format!("Cell with id '{}' not found", id))
                })?;
            pos + 1
        } else {
            // Insert at the beginning if no cell_id specified
            0
        };

        notebook.cells.insert(insert_pos, new_cell);

        // Write back to file
        let new_content =
            serde_json::to_string_pretty(&notebook).map_err(|e| ToolError::Json(e))?;

        fs::write(&path, new_content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to write notebook after insertion to '{}': {}",
                notebook_path, e
            ))
        })?;

        let position_msg = if let Some(id) = cell_id {
            format!("after cell '{}'", id)
        } else {
            "at the beginning".to_string()
        };

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully inserted new {} cell {} in {}",
                cell_type, position_msg, notebook_path
            ),
        ))
    }

    /// Delete cell
    async fn delete_cell(
        &self,
        notebook_path: &str,
        cell_id: &str,
    ) -> Result<ToolResult, ToolError> {
        let path = self.resolve_path(notebook_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        // Read and parse notebook
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read notebook file for deletion '{}': {}",
                notebook_path, e
            ))
        })?;

        let mut notebook: Notebook =
            serde_json::from_str(&content).map_err(|e| ToolError::Json(e))?;

        // Find and remove cell
        let cell_index = notebook
            .cells
            .iter()
            .position(|c| c.id.as_deref() == Some(cell_id))
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!("Cell with id '{}' not found", cell_id))
            })?;

        notebook.cells.remove(cell_index);

        // Write back to file
        let new_content =
            serde_json::to_string_pretty(&notebook).map_err(|e| ToolError::Json(e))?;

        fs::write(&path, new_content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to write notebook after deletion to '{}': {}",
                notebook_path, e
            ))
        })?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully deleted cell '{}' from {}",
                cell_id, notebook_path
            ),
        ))
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
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("notebook_path", "Absolute path to the .ipynb file"),
                ToolParameter::string("new_source", "The new content for the cell"),
                ToolParameter::optional_string(
                    "cell_id",
                    "ID of the cell to edit/delete, or cell after which to insert",
                ),
                ToolParameter::optional_string(
                    "cell_type",
                    "Type of cell: 'code' or 'markdown' (required for insert)",
                ),
                ToolParameter::optional_string(
                    "edit_mode",
                    "Edit operation: 'replace' (default), 'insert', or 'delete'",
                )
                .with_default("replace"),
            ],
        )
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
                self.replace_cell(
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
                self.insert_cell(&notebook_path, cell_id.as_deref(), &cell_type, &new_source)
                    .await?
            }
            "delete" => {
                let cell_id = call.get_string("cell_id").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'cell_id' parameter for delete".to_string(),
                    )
                })?;
                self.delete_cell(&notebook_path, &cell_id).await?
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
        let _notebook_path = call.get_string("notebook_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'notebook_path' parameter".to_string())
        })?;

        let edit_mode = call
            .get_string("edit_mode")
            .unwrap_or_else(|| "replace".to_string());

        match edit_mode.as_str() {
            "replace" => {
                if call.get_string("new_source").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'new_source' parameter for replace".to_string(),
                    ));
                }
                if call.get_string("cell_id").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'cell_id' parameter for replace".to_string(),
                    ));
                }
            }
            "insert" => {
                if call.get_string("new_source").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'new_source' parameter for insert".to_string(),
                    ));
                }
                if call.get_string("cell_type").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'cell_type' parameter for insert".to_string(),
                    ));
                }
            }
            "delete" => {
                if call.get_string("cell_id").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'cell_id' parameter for delete".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown edit_mode: {}. Use 'replace', 'insert', or 'delete'",
                    edit_mode
                )));
            }
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::fs;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    fn create_test_notebook() -> String {
        json!({
            "cells": [
                {
                    "id": "cell-1",
                    "cell_type": "code",
                    "execution_count": null,
                    "metadata": {},
                    "source": ["print('Hello, World!')"],
                    "outputs": []
                },
                {
                    "id": "cell-2",
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# Title\n", "This is markdown"]
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        })
        .to_string()
    }

    #[tokio::test]
    async fn test_notebook_edit_replace_cell() {
        let temp_dir = TempDir::new().unwrap();
        let notebook_path = temp_dir.path().join("test.ipynb");

        // Create test notebook
        fs::write(&notebook_path, create_test_notebook())
            .await
            .unwrap();

        let tool = NotebookEditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-1",
            "notebook_edit",
            json!({
                "notebook_path": notebook_path.to_str().unwrap(),
                "cell_id": "cell-1",
                "new_source": "print('Hello, Rust!')",
                "edit_mode": "replace"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the change
        let content = fs::read_to_string(&notebook_path).await.unwrap();
        let notebook: Notebook = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook.cells.len(), 2);
        let source = NotebookEditTool::source_to_string(&notebook.cells[0].source);
        assert!(source.contains("Hello, Rust!"));
    }

    #[tokio::test]
    async fn test_notebook_edit_insert_cell() {
        let temp_dir = TempDir::new().unwrap();
        let notebook_path = temp_dir.path().join("test.ipynb");

        // Create test notebook
        fs::write(&notebook_path, create_test_notebook())
            .await
            .unwrap();

        let tool = NotebookEditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-2",
            "notebook_edit",
            json!({
                "notebook_path": notebook_path.to_str().unwrap(),
                "cell_id": "cell-1",
                "cell_type": "code",
                "new_source": "x = 42",
                "edit_mode": "insert"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the insertion
        let content = fs::read_to_string(&notebook_path).await.unwrap();
        let notebook: Notebook = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook.cells.len(), 3);
        let source = NotebookEditTool::source_to_string(&notebook.cells[1].source);
        assert!(source.contains("x = 42"));
    }

    #[tokio::test]
    async fn test_notebook_edit_delete_cell() {
        let temp_dir = TempDir::new().unwrap();
        let notebook_path = temp_dir.path().join("test.ipynb");

        // Create test notebook
        fs::write(&notebook_path, create_test_notebook())
            .await
            .unwrap();

        let tool = NotebookEditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-3",
            "notebook_edit",
            json!({
                "notebook_path": notebook_path.to_str().unwrap(),
                "cell_id": "cell-1",
                "edit_mode": "delete"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the deletion
        let content = fs::read_to_string(&notebook_path).await.unwrap();
        let notebook: Notebook = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook.cells.len(), 1);
        assert_eq!(notebook.cells[0].id.as_deref(), Some("cell-2"));
    }

    #[tokio::test]
    async fn test_notebook_edit_cell_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let notebook_path = temp_dir.path().join("test.ipynb");

        // Create test notebook
        fs::write(&notebook_path, create_test_notebook())
            .await
            .unwrap();

        let tool = NotebookEditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "notebook_edit",
            json!({
                "notebook_path": notebook_path.to_str().unwrap(),
                "cell_id": "nonexistent",
                "new_source": "test",
                "edit_mode": "replace"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_notebook_edit_missing_parameters() {
        let tool = NotebookEditTool::new();

        // Missing notebook_path
        let call = create_tool_call(
            "test-5a",
            "notebook_edit",
            json!({
                "cell_id": "cell-1",
                "new_source": "test"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        // Missing new_source for replace
        let call = create_tool_call(
            "test-5b",
            "notebook_edit",
            json!({
                "notebook_path": "/tmp/test.ipynb",
                "cell_id": "cell-1",
                "edit_mode": "replace"
            }),
        );
        let result = tool.validate(&call);
        assert!(result.is_err());

        // Missing cell_type for insert
        let call = create_tool_call(
            "test-5c",
            "notebook_edit",
            json!({
                "notebook_path": "/tmp/test.ipynb",
                "new_source": "test",
                "edit_mode": "insert"
            }),
        );
        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_notebook_edit_insert_at_beginning() {
        let temp_dir = TempDir::new().unwrap();
        let notebook_path = temp_dir.path().join("test.ipynb");

        // Create test notebook
        fs::write(&notebook_path, create_test_notebook())
            .await
            .unwrap();

        let tool = NotebookEditTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-6",
            "notebook_edit",
            json!({
                "notebook_path": notebook_path.to_str().unwrap(),
                "cell_type": "markdown",
                "new_source": "# First Cell",
                "edit_mode": "insert"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the insertion at beginning
        let content = fs::read_to_string(&notebook_path).await.unwrap();
        let notebook: Notebook = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook.cells.len(), 3);
        let source = NotebookEditTool::source_to_string(&notebook.cells[0].source);
        assert!(source.contains("# First Cell"));
    }

    #[test]
    fn test_notebook_edit_schema() {
        let tool = NotebookEditTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "notebook_edit");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_source_conversion() {
        // Test string to source
        let content = "line1\nline2\nline3";
        let source = NotebookEditTool::string_to_source(content);
        assert!(source.is_array());

        // Test source to string
        let result = NotebookEditTool::source_to_string(&source);
        assert_eq!(result, content);
    }
}
