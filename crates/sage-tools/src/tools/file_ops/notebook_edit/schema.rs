//! Schema definition for the notebook edit tool

use sage_core::tools::types::{ToolParameter, ToolSchema};

/// Create the tool schema for notebook editing
pub fn create_schema() -> ToolSchema {
    ToolSchema::new(
        "NotebookEdit",
        "Edit Jupyter notebook (.ipynb) cells. Supports three operations:
- replace: Replace the content of an existing cell (requires cell_id)
- insert: Insert a new cell at a position (requires cell_type; inserts after cell_id if provided, or at beginning)
- delete: Delete an existing cell (requires cell_id)

The notebook_path must be an absolute path to a .ipynb file.
Cell IDs can be found by reading the notebook file first.",
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
