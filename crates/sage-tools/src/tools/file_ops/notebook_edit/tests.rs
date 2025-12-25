//! Tests for notebook edit tool

#![cfg(test)]

use super::NotebookEditTool;
use super::conversion::{source_to_string, string_to_source};
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

use super::types::Notebook;

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
    let source = source_to_string(&notebook.cells[0].source);
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
    let source = source_to_string(&notebook.cells[1].source);
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
    let source = source_to_string(&notebook.cells[0].source);
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
    let source = string_to_source(content);
    assert!(source.is_array());

    // Test source to string
    let result = source_to_string(&source);
    assert_eq!(result, content);
}
