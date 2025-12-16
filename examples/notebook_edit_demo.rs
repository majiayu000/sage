//! Example demonstrating the NotebookEdit tool
//!
//! This example shows how to use the NotebookEdit tool to edit Jupyter notebook cells.

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::NotebookEditTool;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NotebookEdit Tool Demo ===\n");

    // Create a temporary directory for demonstration
    let temp_dir = TempDir::new()?;
    let tool = NotebookEditTool::with_working_directory(temp_dir.path());

    println!("Working directory: {}\n", temp_dir.path().display());

    // Create a sample notebook file
    let notebook_path = temp_dir.path().join("demo.ipynb");
    let sample_notebook = json!({
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
                "source": ["# My Notebook\n", "This is a demo notebook."]
            }
        ],
        "metadata": {},
        "nbformat": 4,
        "nbformat_minor": 5
    });

    fs::write(
        &notebook_path,
        serde_json::to_string_pretty(&sample_notebook)?,
    )
    .await?;
    println!("Created sample notebook: {}\n", notebook_path.display());

    // Example 1: Replace cell content
    println!("1. Replacing content of cell-1...");
    let call = create_tool_call(
        "call-1",
        "notebook_edit",
        json!({
            "notebook_path": notebook_path.to_str().unwrap(),
            "cell_id": "cell-1",
            "new_source": "import numpy as np\nprint('Hello from NumPy!')",
            "edit_mode": "replace"
        }),
    );

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 2: Insert a new cell after cell-1
    println!("\n2. Inserting a new code cell after cell-1...");
    let call = create_tool_call(
        "call-2",
        "notebook_edit",
        json!({
            "notebook_path": notebook_path.to_str().unwrap(),
            "cell_id": "cell-1",
            "cell_type": "code",
            "new_source": "# This is a new cell\nx = 42\nprint(f'The answer is {x}')",
            "edit_mode": "insert"
        }),
    );

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 3: Insert a markdown cell at the beginning
    println!("\n3. Inserting a markdown cell at the beginning...");
    let call = create_tool_call(
        "call-3",
        "notebook_edit",
        json!({
            "notebook_path": notebook_path.to_str().unwrap(),
            "cell_type": "markdown",
            "new_source": "# Introduction\n\nThis notebook demonstrates various Python operations.",
            "edit_mode": "insert"
        }),
    );

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 4: Delete a cell
    println!("\n4. Deleting cell-2...");
    let call = create_tool_call(
        "call-4",
        "notebook_edit",
        json!({
            "notebook_path": notebook_path.to_str().unwrap(),
            "cell_id": "cell-2",
            "new_source": "",  // Required by schema but not used for delete
            "edit_mode": "delete"
        }),
    );

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 5: Try to replace a non-existent cell (should fail)
    println!("\n5. Attempting to replace non-existent cell...");
    let call = create_tool_call(
        "call-5",
        "notebook_edit",
        json!({
            "notebook_path": notebook_path.to_str().unwrap(),
            "cell_id": "nonexistent",
            "new_source": "This should fail!",
            "edit_mode": "replace"
        }),
    );

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Expected error: {}", e);
        }
    }

    println!("\n=== Demo Complete ===");
    println!("\nFinal notebook content:");

    // Read and display the final notebook
    let final_content = fs::read_to_string(&notebook_path).await?;
    let notebook: serde_json::Value = serde_json::from_str(&final_content)?;

    if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
        println!("Total cells: {}", cells.len());
        for (i, cell) in cells.iter().enumerate() {
            let cell_type = cell
                .get("cell_type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let cell_id = cell.get("id").and_then(|id| id.as_str()).unwrap_or("no-id");
            println!("  Cell {}: [{}] id={}", i + 1, cell_type, cell_id);
        }
    }

    Ok(())
}

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
