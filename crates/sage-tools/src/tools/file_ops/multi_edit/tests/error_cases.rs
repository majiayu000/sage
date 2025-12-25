//! Error handling and validation tests

use crate::tools::file_ops::multi_edit::MultiEditTool;
use sage_core::tools::base::Tool;
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

use super::common::create_tool_call;

#[tokio::test]
async fn test_multi_edit_multiple_occurrences_without_replace_all() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "test test test\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-4",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "test", "new_string": "replaced"}
            ]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("appears"));
    }
}

#[tokio::test]
async fn test_multi_edit_string_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-5",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "nonexistent", "new_string": "replacement"}
            ]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("not found"));
    }
}

#[tokio::test]
async fn test_multi_edit_without_reading() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    // Intentionally NOT marking the file as read

    let call = create_tool_call(
        "test-6",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "World", "new_string": "Rust"}
            ]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("has not been read"));
    }
}

#[tokio::test]
async fn test_multi_edit_empty_old_string() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-7",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "", "new_string": "replacement"}
            ]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("empty"));
    }
}

#[tokio::test]
async fn test_multi_edit_identical_strings() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-8",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "World", "new_string": "World"}
            ]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("identical"));
    }
}

#[tokio::test]
async fn test_multi_edit_missing_file_path() {
    let tool = MultiEditTool::new();

    let call = create_tool_call(
        "test-11",
        "MultiEdit",
        json!({
            "edits": [
                {"old_string": "test", "new_string": "replacement"}
            ]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("file_path"));
    }
}

#[tokio::test]
async fn test_multi_edit_missing_edits() {
    let tool = MultiEditTool::new();

    let call = create_tool_call(
        "test-12",
        "MultiEdit",
        json!({
            "file_path": "test.txt"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("edits"));
    }
}

#[tokio::test]
async fn test_multi_edit_empty_edits_array() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-13",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": []
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(err.to_string().contains("at least one"));
    }
}
