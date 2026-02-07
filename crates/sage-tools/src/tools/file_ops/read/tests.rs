//! Tests for the Read tool

use super::tool::ReadTool;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
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

#[tokio::test]
async fn test_read_tool_basic() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create test file
    let content = "Line 1\nLine 2\nLine 3\n";
    fs::write(&file_path, content).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-1",
        "Read",
        json!({
            "file_path": "test.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("     1→Line 1"));
    assert!(output.contains("     2→Line 2"));
    assert!(output.contains("     3→Line 3"));
}

#[tokio::test]
async fn test_read_tool_with_offset() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create test file with multiple lines
    let lines: Vec<String> = (1..=10).map(|i| format!("Line {}", i)).collect();
    fs::write(&file_path, lines.join("\n")).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-2",
        "Read",
        json!({
            "file_path": "test.txt",
            "offset": 5,
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("     6→Line 6")); // offset 5 = line 6 (1-indexed)
    assert!(!output.contains("     5→Line 5"));
}

#[tokio::test]
async fn test_read_tool_with_limit() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create test file with multiple lines
    let lines: Vec<String> = (1..=10).map(|i| format!("Line {}", i)).collect();
    fs::write(&file_path, lines.join("\n")).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-3",
        "Read",
        json!({
            "file_path": "test.txt",
            "limit": 3,
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("     1→Line 1"));
    assert!(output.contains("     2→Line 2"));
    assert!(output.contains("     3→Line 3"));
    assert!(!output.contains("     4→Line 4"));
    assert!(output.contains("truncated")); // Should indicate truncation
}

#[tokio::test]
async fn test_read_tool_with_offset_and_limit() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create test file with multiple lines
    let lines: Vec<String> = (1..=20).map(|i| format!("Line {}", i)).collect();
    fs::write(&file_path, lines.join("\n")).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-4",
        "Read",
        json!({
            "file_path": "test.txt",
            "offset": 10,
            "limit": 5,
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("    11→Line 11")); // offset 10 = line 11
    assert!(output.contains("    15→Line 15"));
    assert!(!output.contains("    10→Line 10"));
    assert!(!output.contains("    16→Line 16"));
}

#[tokio::test]
async fn test_read_tool_truncate_long_lines() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create a file with a very long line
    let long_line = "a".repeat(3000);
    fs::write(&file_path, &long_line).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-5",
        "Read",
        json!({
            "file_path": "test.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("line truncated"));
    assert!(output.contains("3000 chars total"));
}

#[tokio::test]
async fn test_read_tool_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ReadTool::with_working_directory(temp_dir.path());

    let call = create_tool_call(
        "test-6",
        "Read",
        json!({
            "file_path": "nonexistent.txt",
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_read_tool_directory() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-7",
        "Read",
        json!({
            "file_path": "subdir",
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("directory"));
}

#[tokio::test]
async fn test_read_tool_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create test file
    let lines: Vec<String> = (1..=5).map(|i| format!("Line {}", i)).collect();
    fs::write(&file_path, lines.join("\n")).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-8",
        "Read",
        json!({
            "file_path": "test.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    // Check metadata
    assert_eq!(
        result.metadata.get("total_lines").and_then(|v| v.as_u64()),
        Some(5)
    );
    assert_eq!(
        result.metadata.get("lines_read").and_then(|v| v.as_u64()),
        Some(5)
    );
    assert_eq!(
        result.metadata.get("start_line").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        result.metadata.get("end_line").and_then(|v| v.as_u64()),
        Some(5)
    );
    assert_eq!(
        result.metadata.get("truncated").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[tokio::test]
async fn test_read_tool_validation_negative_offset() {
    let tool = ReadTool::new();
    let call = create_tool_call(
        "test-9",
        "Read",
        json!({
            "file_path": "test.txt",
            "offset": -1,
        }),
    );

    let result = tool.validate(&call);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_tool_validation_zero_limit() {
    let tool = ReadTool::new();
    let call = create_tool_call(
        "test-10",
        "Read",
        json!({
            "file_path": "test.txt",
            "limit": 0,
        }),
    );

    let result = tool.validate(&call);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_tool_validation_excessive_limit() {
    let tool = ReadTool::new();
    let call = create_tool_call(
        "test-11",
        "Read",
        json!({
            "file_path": "test.txt",
            "limit": 20000,
        }),
    );

    let result = tool.validate(&call);
    assert!(result.is_err());
}

#[test]
fn test_read_tool_schema() {
    let tool = ReadTool::new();
    let schema = tool.schema();
    assert_eq!(schema.name, "Read");
    assert!(!schema.description.is_empty());
}

#[test]
fn test_read_tool_is_read_only() {
    let tool = ReadTool::new();
    assert!(tool.is_read_only());
}

#[test]
fn test_read_tool_supports_parallel() {
    let tool = ReadTool::new();
    assert!(tool.supports_parallel_execution());
}
