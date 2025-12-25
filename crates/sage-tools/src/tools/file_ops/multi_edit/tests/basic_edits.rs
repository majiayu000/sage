//! Basic edit operation tests

use crate::tools::file_ops::multi_edit::MultiEditTool;
use sage_core::tools::base::Tool;
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

use super::common::create_tool_call;

#[tokio::test]
async fn test_multi_edit_single_edit() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\nThis is a test file.\n")
        .await
        .unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-1",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "World", "new_string": "Rust"}
            ]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("Hello, Rust!"));
    assert!(!content.contains("Hello, World!"));
}

#[tokio::test]
async fn test_multi_edit_multiple_edits() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\nGoodbye, World!\nTest line.\n")
        .await
        .unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-2",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "Hello, World!", "new_string": "Hello, Rust!"},
                {"old_string": "Goodbye, World!", "new_string": "Goodbye, Rust!"},
                {"old_string": "Test line.", "new_string": "Modified line."}
            ]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.as_ref().unwrap().contains("3 edit(s)"));

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("Hello, Rust!"));
    assert!(content.contains("Goodbye, Rust!"));
    assert!(content.contains("Modified line."));
}

#[tokio::test]
async fn test_multi_edit_replace_all() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "foo bar foo baz foo\n")
        .await
        .unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    let call = create_tool_call(
        "test-3",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "foo", "new_string": "qux", "replace_all": true}
            ]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.as_ref().unwrap().contains("3 occurrence(s)"));

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "qux bar qux baz qux\n");
}

#[tokio::test]
async fn test_multi_edit_sequential_edits() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    // First edit changes "World" to "Rust", second edit changes "Rust" to "Universe"
    let call = create_tool_call(
        "test-9",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": "World", "new_string": "Rust"},
                {"old_string": "Rust", "new_string": "Universe"}
            ]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("Hello, Universe!"));
}

#[tokio::test]
async fn test_multi_edit_delete_text() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!\n").await.unwrap();

    let tool = MultiEditTool::with_working_directory(temp_dir.path());
    tool.mark_file_as_read(file_path.clone());

    // Delete ", World" by replacing with empty string
    let call = create_tool_call(
        "test-10",
        "MultiEdit",
        json!({
            "file_path": "test.txt",
            "edits": [
                {"old_string": ", World", "new_string": ""}
            ]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "Hello!\n");
}
