//! Integration tests for the Edit tool

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::file_ops::EditTool;
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
async fn test_edit_tool_basic_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Create initial file
    let initial_content = "Hello, World!\nThis is a test.\nGoodbye!";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    // Perform basic replacement
    println!("\n=== Test 1: Basic string replacement ===");
    let call = create_tool_call(
        "test-1",
        "Edit",
        json!({
            "file_path": "test.txt",
            "old_string": "World",
            "new_string": "Rust"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    // Verify the change
    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("Hello, Rust!"));
    assert!(!content.contains("Hello, World!"));
    println!("✓ Successfully replaced 'World' with 'Rust'");
}

#[tokio::test]
async fn test_edit_tool_multiline_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("multiline.txt");

    // Create file with multiple lines
    let initial_content = "fn main() {\n    println!(\"Hello\");\n}\n";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 2: Multiline replacement ===");
    let call = create_tool_call(
        "test-2",
        "Edit",
        json!({
            "file_path": "multiline.txt",
            "old_string": "fn main() {\n    println!(\"Hello\");\n}",
            "new_string": "fn main() {\n    println!(\"Hello, World!\");\n    println!(\"Modified\");\n}"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("println!(\"Hello, World!\")"));
    assert!(content.contains("println!(\"Modified\")"));
    println!("✓ Successfully performed multiline replacement");
}

#[tokio::test]
async fn test_edit_tool_replace_all() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("replace_all.txt");

    // Create file with multiple occurrences
    let initial_content = "test test test\nanother test\nfinal test";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 3: Replace all occurrences ===");
    let call = create_tool_call(
        "test-3",
        "Edit",
        json!({
            "file_path": "replace_all.txt",
            "old_string": "test",
            "new_string": "demo",
            "replace_all": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "demo demo demo\nanother demo\nfinal demo");
    assert!(!content.contains("test"));

    if let Some(output) = &result.output {
        assert!(output.contains("5 occurrences"));
    }
    println!("✓ Successfully replaced all 5 occurrences");
}

#[tokio::test]
async fn test_edit_tool_preserve_indentation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("indentation.py");

    // Create Python file with indentation
    let initial_content = "def hello():\n    print(\"hello\")\n    return True";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 4: Preserve indentation ===");
    let call = create_tool_call(
        "test-4",
        "Edit",
        json!({
            "file_path": "indentation.py",
            "old_string": "    print(\"hello\")",
            "new_string": "    print(\"Hello, World!\")"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    // Verify indentation is preserved
    assert!(content.contains("    print(\"Hello, World!\")"));
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines[1].starts_with("    "));
    println!("✓ Successfully preserved indentation");
}

#[tokio::test]
async fn test_edit_tool_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("special.txt");

    // Create file with special characters
    let initial_content = "Symbols: $VAR, @user, #tag, 100%";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 5: Handle special characters ===");
    let call = create_tool_call(
        "test-5",
        "Edit",
        json!({
            "file_path": "special.txt",
            "old_string": "$VAR, @user, #tag",
            "new_string": "${VARIABLE}, @username, #hashtag"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("${VARIABLE}"));
    assert!(content.contains("@username"));
    assert!(content.contains("#hashtag"));
    println!("✓ Successfully handled special characters");
}

#[tokio::test]
async fn test_edit_tool_error_string_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!").await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 6: Error - String not found ===");
    let call = create_tool_call(
        "test-6",
        "Edit",
        json!({
            "file_path": "test.txt",
            "old_string": "NonexistentString",
            "new_string": "replacement"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
    println!("✓ Correctly detected string not found");
}

#[tokio::test]
async fn test_edit_tool_error_multiple_occurrences() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "test test test").await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 7: Error - Multiple occurrences without replace_all ===");
    let call = create_tool_call(
        "test-7",
        "Edit",
        json!({
            "file_path": "test.txt",
            "old_string": "test",
            "new_string": "replaced"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("3 occurrences"));
    assert!(err.to_string().contains("replace_all"));
    println!("✓ Correctly detected multiple occurrences");
}

#[tokio::test]
async fn test_edit_tool_error_same_strings() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!").await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 8: Error - Same old and new strings ===");
    let call = create_tool_call(
        "test-8",
        "Edit",
        json!({
            "file_path": "test.txt",
            "old_string": "World",
            "new_string": "World"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("same"));
    println!("✓ Correctly detected identical strings");
}

#[tokio::test]
async fn test_edit_tool_error_empty_old_string() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!").await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 9: Error - Empty old_string ===");
    let call = create_tool_call(
        "test-9",
        "Edit",
        json!({
            "file_path": "test.txt",
            "old_string": "",
            "new_string": "something"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("empty"));
    println!("✓ Correctly detected empty old_string");
}

#[tokio::test]
async fn test_edit_tool_error_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 10: Error - File not found ===");
    let call = create_tool_call(
        "test-10",
        "Edit",
        json!({
            "file_path": "nonexistent.txt",
            "old_string": "old",
            "new_string": "new"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
    println!("✓ Correctly detected file not found");
}

#[tokio::test]
async fn test_edit_tool_code_refactoring() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("code.rs");

    // Create a Rust source file
    let initial_content = r#"fn calculate(x: i32) -> i32 {
    let result = x * 2;
    result
}

fn main() {
    let value = calculate(5);
    println!("{}", value);
}"#;
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 11: Code refactoring ===");

    // Refactor: change function signature
    let call = create_tool_call(
        "test-11a",
        "Edit",
        json!({
            "file_path": "code.rs",
            "old_string": "fn calculate(x: i32) -> i32",
            "new_string": "fn calculate(x: i32, multiplier: i32) -> i32"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    // Refactor: change implementation
    let call = create_tool_call(
        "test-11b",
        "Edit",
        json!({
            "file_path": "code.rs",
            "old_string": "    let result = x * 2;",
            "new_string": "    let result = x * multiplier;"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("fn calculate(x: i32, multiplier: i32) -> i32"));
    assert!(content.contains("let result = x * multiplier;"));
    println!("✓ Successfully refactored code");
}

#[tokio::test]
async fn test_edit_tool_json_modification() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.json");

    // Create JSON configuration
    let initial_content = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "debug": false
}"#;
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 12: JSON modification ===");
    let call = create_tool_call(
        "test-12",
        "Edit",
        json!({
            "file_path": "config.json",
            "old_string": "  \"version\": \"1.0.0\",",
            "new_string": "  \"version\": \"2.0.0\","
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("\"version\": \"2.0.0\""));

    // Verify JSON is still valid
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], "2.0.0");
    println!("✓ Successfully modified JSON");
}

#[tokio::test]
async fn test_edit_tool_unicode_content() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("unicode.txt");

    // Create file with Unicode content
    let initial_content = "你好，世界！\nHello, World!\nこんにちは世界！";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 13: Unicode content ===");
    let call = create_tool_call(
        "test-13",
        "Edit",
        json!({
            "file_path": "unicode.txt",
            "old_string": "你好，世界！",
            "new_string": "你好，Rust！"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("你好，Rust！"));
    assert!(!content.contains("你好，世界！"));
    println!("✓ Successfully handled Unicode content");
}

#[tokio::test]
async fn test_edit_tool_newline_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("newlines.txt");

    // Create file with specific newline pattern
    let initial_content = "Line 1\n\nLine 3\nLine 4";
    fs::write(&file_path, initial_content).await.unwrap();

    let tool = EditTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 14: Newline preservation ===");
    let call = create_tool_call(
        "test-14",
        "Edit",
        json!({
            "file_path": "newlines.txt",
            "old_string": "Line 1\n\nLine 3",
            "new_string": "Line 1\n\nLine 2\nLine 3"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 5); // Line 1, "", Line 2, Line 3, Line 4
    assert_eq!(lines[0], "Line 1");
    assert_eq!(lines[1], "");
    assert_eq!(lines[2], "Line 2");
    assert_eq!(lines[3], "Line 3");
    assert_eq!(lines[4], "Line 4");
    println!("✓ Successfully preserved newline structure");
}
