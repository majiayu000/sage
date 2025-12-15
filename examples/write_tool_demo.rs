//! Example demonstrating the Write tool
//!
//! This example shows how to use the Write tool to create and overwrite files.

use sage_tools::WriteTool;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use std::collections::HashMap;
use serde_json::json;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Write Tool Demo ===\n");

    // Create a temporary directory for demonstration
    let temp_dir = TempDir::new()?;
    let tool = WriteTool::with_working_directory(temp_dir.path());

    println!("Working directory: {}\n", temp_dir.path().display());

    // Example 1: Create a new file
    println!("1. Creating a new file...");
    let call = create_tool_call("call-1", "Write", json!({
        "file_path": "hello.txt",
        "content": "Hello, World!\nThis is a test file."
    }));

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 2: Create a file in a nested directory
    println!("\n2. Creating a file in nested directories...");
    let call = create_tool_call("call-2", "Write", json!({
        "file_path": "subdir/nested/file.txt",
        "content": "This file is in a nested directory structure."
    }));

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 3: Try to overwrite without reading (should fail)
    println!("\n3. Attempting to overwrite existing file without reading first...");
    let call = create_tool_call("call-3", "Write", json!({
        "file_path": "hello.txt",
        "content": "This should fail!"
    }));

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Expected error: {}", e);
        }
    }

    // Example 4: Mark file as read and then overwrite
    println!("\n4. Marking file as read, then overwriting...");
    let file_path = temp_dir.path().join("hello.txt");
    tool.mark_file_as_read(file_path);

    let call = create_tool_call("call-4", "Write", json!({
        "file_path": "hello.txt",
        "content": "Updated content after reading!"
    }));

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 5: Write multiline content
    println!("\n5. Writing multiline content...");
    let multiline_content = r#"# README

This is a sample markdown file.

## Features
- Feature 1
- Feature 2
- Feature 3

## Usage
Run the program with `cargo run`.
"#;

    let call = create_tool_call("call-5", "Write", json!({
        "file_path": "README.md",
        "content": multiline_content
    }));

    match tool.execute(&call).await {
        Ok(result) => {
            println!("   Success: {}", result.output.unwrap_or_default());
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    println!("\n=== Demo Complete ===");
    println!("\nFiles created in: {}", temp_dir.path().display());

    // List created files
    println!("\nCreated files:");
    if let Ok(entries) = std::fs::read_dir(temp_dir.path()) {
        for entry in entries.flatten() {
            println!("  - {}", entry.path().display());
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
