//! Notebook cell operations (replace, insert, delete)

use sage_core::tools::base::{FileSystemTool, ToolError};
use sage_core::tools::types::ToolResult;
use std::path::Path;
use tokio::fs;

use super::conversion::string_to_source;
use super::types::{Notebook, NotebookCell};

/// Replace cell content in a notebook
pub async fn replace_cell(
    tool: &dyn FileSystemTool,
    tool_name: &str,
    notebook_path: &str,
    cell_id: Option<&str>,
    new_source: &str,
    cell_type: Option<&str>,
) -> Result<ToolResult, ToolError> {
    let path = tool.resolve_path(notebook_path);

    // Security check
    if !tool.is_safe_path(&path) {
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

    let mut notebook: Notebook = serde_json::from_str(&content).map_err(|e| ToolError::Json(e))?;

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
    cell.source = string_to_source(new_source);

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
    write_notebook(&path, &notebook, notebook_path).await?;

    Ok(ToolResult::success(
        "",
        tool_name,
        format!(
            "Successfully replaced content of cell '{}' in {}",
            cell_id.unwrap_or("unknown"),
            notebook_path
        ),
    ))
}

/// Insert new cell into a notebook
pub async fn insert_cell(
    tool: &dyn FileSystemTool,
    tool_name: &str,
    notebook_path: &str,
    cell_id: Option<&str>,
    cell_type: &str,
    new_source: &str,
) -> Result<ToolResult, ToolError> {
    let path = tool.resolve_path(notebook_path);

    // Security check
    if !tool.is_safe_path(&path) {
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

    let mut notebook: Notebook = serde_json::from_str(&content).map_err(|e| ToolError::Json(e))?;

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
        source: string_to_source(new_source),
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
    write_notebook(&path, &notebook, notebook_path).await?;

    let position_msg = if let Some(id) = cell_id {
        format!("after cell '{}'", id)
    } else {
        "at the beginning".to_string()
    };

    Ok(ToolResult::success(
        "",
        tool_name,
        format!(
            "Successfully inserted new {} cell {} in {}",
            cell_type, position_msg, notebook_path
        ),
    ))
}

/// Delete cell from a notebook
pub async fn delete_cell(
    tool: &dyn FileSystemTool,
    tool_name: &str,
    notebook_path: &str,
    cell_id: &str,
) -> Result<ToolResult, ToolError> {
    let path = tool.resolve_path(notebook_path);

    // Security check
    if !tool.is_safe_path(&path) {
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

    let mut notebook: Notebook = serde_json::from_str(&content).map_err(|e| ToolError::Json(e))?;

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
    write_notebook(&path, &notebook, notebook_path).await?;

    Ok(ToolResult::success(
        "",
        tool_name,
        format!(
            "Successfully deleted cell '{}' from {}",
            cell_id, notebook_path
        ),
    ))
}

/// Helper function to write notebook to file
async fn write_notebook(
    path: &Path,
    notebook: &Notebook,
    notebook_path: &str,
) -> Result<(), ToolError> {
    let new_content = serde_json::to_string_pretty(notebook).map_err(|e| ToolError::Json(e))?;

    fs::write(path, new_content).await.map_err(|e| {
        ToolError::ExecutionFailed(format!(
            "Failed to write updated notebook to '{}': {}",
            notebook_path, e
        ))
    })?;

    Ok(())
}
