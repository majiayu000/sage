//! Demo of the Read tool functionality

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::file_ops::ReadTool;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Read Tool Demo ===\n");

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("read_tool_test.txt");

    // Write some test content
    let content = (1..=50)
        .map(|i| format!("Line {} - This is a test line with some content", i))
        .collect::<Vec<String>>()
        .join("\n");

    tokio::fs::write(&test_file, content).await?;
    println!("Created test file: {}", test_file.display());
    println!("File contains 50 lines\n");

    // Create the Read tool
    let tool = ReadTool::new();

    // Test 1: Read entire file (default)
    println!("--- Test 1: Read entire file (first 50 lines) ---");
    let call1 = ToolCall {
        id: "test-1".to_string(),
        name: "Read".to_string(),
        arguments: HashMap::from([(
            "file_path".to_string(),
            json!(test_file.to_string_lossy().to_string()),
        )]),
        call_id: None,
    };

    let result1 = tool.execute(&call1).await?;
    println!("Success: {}", result1.success);
    if let Some(output) = &result1.output {
        let lines: Vec<&str> = output.lines().collect();
        println!("Output (first 5 lines):");
        for line in lines.iter().take(5) {
            println!("{}", line);
        }
        println!("... ({} total lines in output)", lines.len());
    }
    println!("Metadata: {:?}\n", result1.metadata);

    // Test 2: Read with offset
    println!("--- Test 2: Read with offset (skip first 20 lines) ---");
    let call2 = ToolCall {
        id: "test-2".to_string(),
        name: "Read".to_string(),
        arguments: HashMap::from([
            (
                "file_path".to_string(),
                json!(test_file.to_string_lossy().to_string()),
            ),
            ("offset".to_string(), json!(20)),
        ]),
        call_id: None,
    };

    let result2 = tool.execute(&call2).await?;
    if let Some(output) = &result2.output {
        let lines: Vec<&str> = output.lines().collect();
        println!("First 3 lines after offset:");
        for line in lines.iter().take(3) {
            println!("{}", line);
        }
    }
    println!("Metadata: {:?}\n", result2.metadata);

    // Test 3: Read with limit
    println!("--- Test 3: Read with limit (first 10 lines only) ---");
    let call3 = ToolCall {
        id: "test-3".to_string(),
        name: "Read".to_string(),
        arguments: HashMap::from([
            (
                "file_path".to_string(),
                json!(test_file.to_string_lossy().to_string()),
            ),
            ("limit".to_string(), json!(10)),
        ]),
        call_id: None,
    };

    let result3 = tool.execute(&call3).await?;
    if let Some(output) = &result3.output {
        let lines: Vec<&str> = output.lines().collect();
        println!("All output lines ({} lines):", lines.len());
        for line in lines.iter() {
            println!("{}", line);
        }
    }
    println!("Metadata: {:?}\n", result3.metadata);

    // Test 4: Read with offset and limit
    println!("--- Test 4: Read with offset and limit (lines 30-35) ---");
    let call4 = ToolCall {
        id: "test-4".to_string(),
        name: "Read".to_string(),
        arguments: HashMap::from([
            (
                "file_path".to_string(),
                json!(test_file.to_string_lossy().to_string()),
            ),
            ("offset".to_string(), json!(29)), // 0-indexed, so 29 = line 30
            ("limit".to_string(), json!(6)),
        ]),
        call_id: None,
    };

    let result4 = tool.execute(&call4).await?;
    if let Some(output) = &result4.output {
        println!("Output:");
        println!("{}", output);
    }
    println!("Metadata: {:?}\n", result4.metadata);

    // Clean up
    tokio::fs::remove_file(&test_file).await?;
    println!("Test file cleaned up.");

    Ok(())
}
