//! Integration tests for the Read tool

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::file_ops::ReadTool;
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
async fn test_read_tool_comprehensive() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("comprehensive_test.txt");

    // Create a file with 100 lines
    let lines: Vec<String> = (1..=100)
        .map(|i| format!("Line {} - Test content with some data", i))
        .collect();
    fs::write(&file_path, lines.join("\n")).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());

    // Test 1: Read entire file (should be truncated at 2000 lines in normal use, but we only have 100)
    println!("\n=== Test 1: Read entire file ===");
    let call = create_tool_call(
        "test-1",
        "Read",
        json!({
            "file_path": "comprehensive_test.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert_eq!(
        result.metadata.get("total_lines").and_then(|v| v.as_u64()),
        Some(100)
    );
    assert_eq!(
        result.metadata.get("truncated").and_then(|v| v.as_bool()),
        Some(false)
    );
    println!("✓ Successfully read all 100 lines");

    // Test 2: Read with offset
    println!("\n=== Test 2: Read with offset ===");
    let call = create_tool_call(
        "test-2",
        "Read",
        json!({
            "file_path": "comprehensive_test.txt",
            "offset": 50,
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert_eq!(
        result.metadata.get("start_line").and_then(|v| v.as_u64()),
        Some(51)
    );
    assert_eq!(
        result.metadata.get("lines_read").and_then(|v| v.as_u64()),
        Some(50)
    );

    if let Some(output) = &result.output {
        assert!(output.contains("    51→Line 51"));
        assert!(output.contains("   100→Line 100"));
    }
    println!("✓ Successfully read lines 51-100");

    // Test 3: Read with limit
    println!("\n=== Test 3: Read with limit ===");
    let call = create_tool_call(
        "test-3",
        "Read",
        json!({
            "file_path": "comprehensive_test.txt",
            "limit": 25,
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert_eq!(
        result.metadata.get("lines_read").and_then(|v| v.as_u64()),
        Some(25)
    );
    assert_eq!(
        result.metadata.get("truncated").and_then(|v| v.as_bool()),
        Some(true)
    );

    if let Some(output) = &result.output {
        assert!(output.contains("truncated"));
        assert!(output.contains("showing lines 1-25 of 100 total lines"));
    }
    println!("✓ Successfully read first 25 lines with truncation notice");

    // Test 4: Read with offset and limit (pagination)
    println!("\n=== Test 4: Read with offset and limit (pagination) ===");
    let call = create_tool_call(
        "test-4",
        "Read",
        json!({
            "file_path": "comprehensive_test.txt",
            "offset": 40,
            "limit": 20,
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert_eq!(
        result.metadata.get("start_line").and_then(|v| v.as_u64()),
        Some(41)
    );
    assert_eq!(
        result.metadata.get("end_line").and_then(|v| v.as_u64()),
        Some(60)
    );
    assert_eq!(
        result.metadata.get("lines_read").and_then(|v| v.as_u64()),
        Some(20)
    );

    if let Some(output) = &result.output {
        assert!(output.contains("    41→Line 41"));
        assert!(output.contains("    60→Line 60"));
        assert!(!output.contains("    40→Line 40"));
        assert!(!output.contains("    61→Line 61"));
    }
    println!("✓ Successfully read lines 41-60");
}

#[tokio::test]
async fn test_read_tool_line_truncation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("long_lines.txt");

    // Create a file with very long lines
    let short_line = "Short line";
    let long_line = "x".repeat(2500); // Exceeds MAX_LINE_LENGTH
    let content = format!("{}\n{}\n{}", short_line, long_line, short_line);

    fs::write(&file_path, content).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-long",
        "Read",
        json!({
            "file_path": "long_lines.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("Short line"));
        assert!(output.contains("line truncated"));
        assert!(output.contains("2500 chars total"));
        // Verify the long line is actually truncated
        assert!(!output.contains(&"x".repeat(2500)));
    }
    println!("✓ Long lines properly truncated with notification");
}

#[tokio::test]
async fn test_read_tool_binary_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Test image file
    let image_path = temp_dir.path().join("test.png");
    fs::write(&image_path, vec![0x89, 0x50, 0x4E, 0x47])
        .await
        .unwrap(); // PNG header

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-img",
        "Read",
        json!({
            "file_path": "test.png",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("Image file detected"));
        assert!(output.contains("PNG"));
        assert!(output.contains("Binary content cannot be displayed"));
    }
    println!("✓ PNG file properly detected");

    // Test PDF file
    let pdf_path = temp_dir.path().join("test.pdf");
    fs::write(&pdf_path, b"%PDF-1.4").await.unwrap();

    let call = create_tool_call(
        "test-pdf",
        "Read",
        json!({
            "file_path": "test.pdf",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("PDF file detected"));
        assert!(output.contains("Binary content cannot be displayed"));
    }
    println!("✓ PDF file properly detected");
}

#[tokio::test]
async fn test_read_tool_line_number_formatting() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("formatting_test.txt");

    // Create file with lines that test formatting
    let content = "First line\nSecond line\nThird line";
    fs::write(&file_path, content).await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-fmt",
        "Read",
        json!({
            "file_path": "formatting_test.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        // Verify line number format: "   1→content"
        assert!(output.contains("     1→First line"));
        assert!(output.contains("     2→Second line"));
        assert!(output.contains("     3→Third line"));

        // Verify no leading zeros or other formatting issues
        assert!(!output.contains("01→"));
        assert!(!output.contains("001→"));
    }
    println!("✓ Line numbers properly formatted");
}

#[tokio::test]
async fn test_read_tool_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.txt");

    fs::write(&file_path, "").await.unwrap();

    let tool = ReadTool::with_working_directory(temp_dir.path());
    let call = create_tool_call(
        "test-empty",
        "Read",
        json!({
            "file_path": "empty.txt",
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert_eq!(
        result.metadata.get("total_lines").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        result.metadata.get("lines_read").and_then(|v| v.as_u64()),
        Some(0)
    );
    println!("✓ Empty file handled correctly");
}

#[tokio::test]
async fn test_read_tool_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ReadTool::with_working_directory(temp_dir.path());

    // Test non-existent file
    let call = create_tool_call(
        "test-404",
        "Read",
        json!({
            "file_path": "nonexistent.txt",
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
    println!("✓ Non-existent file error handled");

    // Test directory
    let dir_path = temp_dir.path().join("testdir");
    fs::create_dir(&dir_path).await.unwrap();

    let call = create_tool_call(
        "test-dir",
        "Read",
        json!({
            "file_path": "testdir",
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("directory"));
    println!("✓ Directory error handled");

    // Test invalid offset
    let file_path = temp_dir.path().join("small.txt");
    fs::write(&file_path, "Line 1\nLine 2").await.unwrap();

    let call = create_tool_call(
        "test-offset",
        "Read",
        json!({
            "file_path": "small.txt",
            "offset": 100,
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("exceeds total lines")
    );
    println!("✓ Invalid offset error handled");
}
