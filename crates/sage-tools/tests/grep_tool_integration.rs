//! Integration tests for the Grep tool

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::file_ops::GrepTool;
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
async fn test_grep_basic_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple test files
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");
    let file3 = temp_dir.path().join("file3.txt");

    fs::write(
        &file1,
        "Hello World\nThis contains the keyword test\nAnother line",
    )
    .await
    .unwrap();
    fs::write(&file2, "No matches here\nJust plain text")
        .await
        .unwrap();
    fs::write(&file3, "Testing again\ntest test test")
        .await
        .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 1: Basic search - files_with_matches mode ===");
    let call = create_tool_call(
        "test-1",
        "Grep",
        json!({
            "pattern": "test",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("file1.txt"));
        assert!(!output.contains("file2.txt"));
        assert!(output.contains("file3.txt"));
        assert!(output.contains("2 file(s) with matches"));
    }
    println!("✓ Found matches in 2 files");
}

#[tokio::test]
async fn test_grep_content_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("code.rs");

    let content = r#"fn main() {
    println!("Hello");
}

fn test_function() {
    println!("Testing");
}

fn another_test() {
    println!("More tests");
}"#;
    fs::write(&file, content).await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 2: Content mode with line numbers ===");
    let call = create_tool_call(
        "test-2",
        "Grep",
        json!({
            "pattern": "fn.*\\(\\)",
            "output_mode": "content",
            "-n": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("fn main()"));
        assert!(output.contains("fn test_function()"));
        assert!(output.contains("fn another_test()"));
        assert!(output.contains("1:"));
        assert!(output.contains("5:"));
        assert!(output.contains("9:"));
    }
    println!("✓ Found all function declarations with line numbers");
}

#[tokio::test]
async fn test_grep_case_insensitive() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.txt");

    fs::write(&file, "Hello World\nhello world\nHELLO WORLD\nheLLo WoRLd")
        .await
        .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 3: Case insensitive search ===");
    let call = create_tool_call(
        "test-3",
        "Grep",
        json!({
            "pattern": "hello",
            "-i": true,
            "output_mode": "count"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("test.txt:4"));
    }
    println!("✓ Case insensitive search matched all 4 lines");
}

#[tokio::test]
async fn test_grep_with_context_lines() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("context.txt");

    fs::write(
        &file,
        "Line 1\nLine 2\nLine 3 MATCH\nLine 4\nLine 5\nLine 6\nLine 7 MATCH\nLine 8\nLine 9",
    )
    .await
    .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 4: Context lines (-A and -B) ===");
    let call = create_tool_call(
        "test-4",
        "Grep",
        json!({
            "pattern": "MATCH",
            "output_mode": "content",
            "-A": 1,
            "-B": 1,
            "-n": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        // First match context
        assert!(output.contains("2:\tLine 2"));
        assert!(output.contains("3:\tLine 3 MATCH"));
        assert!(output.contains("4:\tLine 4"));

        // Second match context
        assert!(output.contains("6:\tLine 6"));
        assert!(output.contains("7:\tLine 7 MATCH"));
        assert!(output.contains("8:\tLine 8"));
    }
    println!("✓ Context lines displayed correctly");
}

#[tokio::test]
async fn test_grep_context_combined() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("combined.txt");

    fs::write(&file, "Line 1\nLine 2\nMATCH\nLine 4\nLine 5")
        .await
        .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 5: Combined context (-C) ===");
    let call = create_tool_call(
        "test-5",
        "Grep",
        json!({
            "pattern": "MATCH",
            "output_mode": "content",
            "-C": 2,
            "-n": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("1:\tLine 1"));
        assert!(output.contains("2:\tLine 2"));
        assert!(output.contains("3:\tMATCH"));
        assert!(output.contains("4:\tLine 4"));
        assert!(output.contains("5:\tLine 5"));
    }
    println!("✓ Combined context (-C 2) showed 2 lines before and after");
}

#[tokio::test]
async fn test_grep_glob_filter() {
    let temp_dir = TempDir::new().unwrap();

    // Create files with different extensions
    let rust_file = temp_dir.path().join("code.rs");
    let txt_file = temp_dir.path().join("doc.txt");
    let py_file = temp_dir.path().join("script.py");

    let pattern_text = "pattern";
    fs::write(&rust_file, pattern_text).await.unwrap();
    fs::write(&txt_file, pattern_text).await.unwrap();
    fs::write(&py_file, pattern_text).await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 6: Glob filter *.rs ===");
    let call = create_tool_call(
        "test-6",
        "Grep",
        json!({
            "pattern": "pattern",
            "glob": "*.rs",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("code.rs"));
        assert!(!output.contains("doc.txt"));
        assert!(!output.contains("script.py"));
    }
    println!("✓ Glob filter correctly matched only .rs files");
}

#[tokio::test]
async fn test_grep_type_filter() {
    let temp_dir = TempDir::new().unwrap();

    // Create files with different types
    let rs_file = temp_dir.path().join("main.rs");
    let py_file = temp_dir.path().join("main.py");
    let js_file = temp_dir.path().join("main.js");

    let content = "main function";
    fs::write(&rs_file, content).await.unwrap();
    fs::write(&py_file, content).await.unwrap();
    fs::write(&js_file, content).await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 7: Type filter (rust) ===");
    let call = create_tool_call(
        "test-7",
        "Grep",
        json!({
            "pattern": "main",
            "type": "rust",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("main.rs"));
        assert!(!output.contains("main.py"));
        assert!(!output.contains("main.js"));
    }
    println!("✓ Type filter correctly matched only Rust files");
}

#[tokio::test]
async fn test_grep_count_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");

    fs::write(&file1, "match\nmatch\nmatch").await.unwrap();
    fs::write(&file2, "match\nmatch").await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 8: Count mode ===");
    let call = create_tool_call(
        "test-8",
        "Grep",
        json!({
            "pattern": "match",
            "output_mode": "count"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("file1.txt:3"));
        assert!(output.contains("file2.txt:2"));
        assert!(output.contains("Total matches: 2"));
    }
    println!("✓ Count mode showed match counts per file");
}

#[tokio::test]
async fn test_grep_head_limit() {
    let temp_dir = TempDir::new().unwrap();

    // Create 10 files with matches
    for i in 1..=10 {
        let file = temp_dir.path().join(format!("file{}.txt", i));
        fs::write(&file, "keyword").await.unwrap();
    }

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 9: Head limit ===");
    let call = create_tool_call(
        "test-9",
        "Grep",
        json!({
            "pattern": "keyword",
            "output_mode": "files_with_matches",
            "head_limit": 3
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        // Count only lines that contain "file" and ".txt", excluding summary line
        let line_count = output
            .lines()
            .filter(|l| l.contains("file") && l.contains(".txt"))
            .count();
        assert_eq!(line_count, 3);
    }
    println!("✓ Head limit correctly limited results to 3 files");
}

#[tokio::test]
async fn test_grep_offset() {
    let temp_dir = TempDir::new().unwrap();

    // Create 5 files
    for i in 1..=5 {
        let file = temp_dir.path().join(format!("file{}.txt", i));
        fs::write(&file, "match").await.unwrap();
    }

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 10: Offset ===");
    let call = create_tool_call(
        "test-10",
        "Grep",
        json!({
            "pattern": "match",
            "output_mode": "files_with_matches",
            "offset": 2,
            "head_limit": 2
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        // Should skip first 2 files and show next 2
        let lines: Vec<&str> = output
            .lines()
            .filter(|l| l.contains("file") && l.contains(".txt"))
            .collect();
        assert_eq!(lines.len(), 2);
    }
    println!("✓ Offset correctly skipped first 2 results");
}

#[tokio::test]
async fn test_grep_regex_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("regex.txt");

    fs::write(
        &file,
        "email@example.com\nuser@test.org\nnot-an-email\n123-456-7890",
    )
    .await
    .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 11: Email regex pattern ===");
    let call = create_tool_call(
        "test-11",
        "Grep",
        json!({
            "pattern": r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
            "output_mode": "count"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("regex.txt:2"));
    }
    println!("✓ Regex pattern matched 2 email addresses");

    println!("\n=== Test 12: Phone number regex pattern ===");
    let call = create_tool_call(
        "test-12",
        "Grep",
        json!({
            "pattern": r"\d{3}-\d{3}-\d{4}",
            "output_mode": "count"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("regex.txt:1"));
    }
    println!("✓ Regex pattern matched 1 phone number");
}

#[tokio::test]
async fn test_grep_multiline_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("multiline.txt");

    // Create file where pattern exists within single lines
    fs::write(
        &file,
        "start middle end\nsomething else\nstart and middle again",
    )
    .await
    .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 13: Multiline mode with dot matches ===");
    let call = create_tool_call(
        "test-13",
        "Grep",
        json!({
            "pattern": "start.*middle",
            "multiline": true,
            "output_mode": "count"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        // Should match 2 lines: "start middle end" and "start and middle again"
        assert!(output.contains("multiline.txt:2"));
    }
    println!("✓ Multiline mode matched pattern with dot operator");
}

#[tokio::test]
async fn test_grep_no_matches() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.txt");

    fs::write(&file, "Some content\nNo matches here")
        .await
        .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 14: No matches found ===");
    let call = create_tool_call(
        "test-14",
        "Grep",
        json!({
            "pattern": "nonexistent",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("No matches found"));
    }
    println!("✓ Correctly reported no matches");
}

#[tokio::test]
async fn test_grep_invalid_regex() {
    let temp_dir = TempDir::new().unwrap();
    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 15: Invalid regex pattern ===");
    let call = create_tool_call(
        "test-15",
        "Grep",
        json!({
            "pattern": "[invalid(regex",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid regex"));
    println!("✓ Invalid regex correctly detected");
}

#[tokio::test]
async fn test_grep_directory_search() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).await.unwrap();

    let file1 = temp_dir.path().join("root.txt");
    let file2 = subdir.join("nested.txt");

    fs::write(&file1, "keyword").await.unwrap();
    fs::write(&file2, "keyword").await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 16: Directory search (recursive) ===");
    let call = create_tool_call(
        "test-16",
        "Grep",
        json!({
            "pattern": "keyword",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("root.txt"));
        assert!(output.contains("nested.txt"));
    }
    println!("✓ Recursive search found files in subdirectories");
}

#[tokio::test]
async fn test_grep_specific_file() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("target.txt");
    let file2 = temp_dir.path().join("other.txt");

    fs::write(&file1, "match").await.unwrap();
    fs::write(&file2, "match").await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 17: Search specific file ===");
    let call = create_tool_call(
        "test-17",
        "Grep",
        json!({
            "pattern": "match",
            "path": "target.txt",
            "output_mode": "files_with_matches"
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("target.txt"));
        assert!(!output.contains("other.txt"));
    }
    println!("✓ Search limited to specific file");
}

#[tokio::test]
async fn test_grep_code_search() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("code.rs");

    let content = r#"use std::io;

pub fn process_data(input: &str) -> Result<String, io::Error> {
    // TODO: implement this
    let result = input.trim();
    Ok(result.to_string())
}

// FIXME: handle edge cases
fn validate(data: &str) -> bool {
    !data.is_empty()
}

// TODO: add tests
"#;
    fs::write(&file, content).await.unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 18: Find TODO comments ===");
    let call = create_tool_call(
        "test-18",
        "Grep",
        json!({
            "pattern": "TODO:",
            "output_mode": "content",
            "-n": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("TODO: implement this"));
        assert!(output.contains("TODO: add tests"));
        assert!(output.contains("4:"));
        assert!(output.contains("14:")); // Second TODO is on line 14, not 13
    }
    println!("✓ Found all TODO comments");

    println!("\n=== Test 19: Find function definitions ===");
    let call = create_tool_call(
        "test-19",
        "Grep",
        json!({
            "pattern": r"^(pub )?fn \w+",
            "output_mode": "content",
            "-n": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("pub fn process_data"));
        assert!(output.contains("fn validate"));
    }
    println!("✓ Found all function definitions");
}

#[tokio::test]
async fn test_grep_without_line_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.txt");

    fs::write(&file, "Line 1\nLine 2 match\nLine 3")
        .await
        .unwrap();

    let tool = GrepTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 20: Content without line numbers ===");
    let call = create_tool_call(
        "test-20",
        "Grep",
        json!({
            "pattern": "match",
            "output_mode": "content",
            "-n": false
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    if let Some(output) = &result.output {
        assert!(output.contains("Line 2 match"));
        // Should not contain line number prefix like "2:"
        assert!(!output.contains("2:"));
    }
    println!("✓ Output without line numbers");
}
