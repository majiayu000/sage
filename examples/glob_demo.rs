//! Example demonstrating the Glob tool for file pattern matching
//!
//! Run with: cargo run --example glob_demo

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::GlobTool;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the Glob tool
    let glob_tool = GlobTool::new();

    println!("=== Glob Tool Demo ===\n");
    println!("Tool: {}", glob_tool.name());
    println!("Description: {}\n", glob_tool.description());

    // Example 1: Find all Rust files recursively
    println!("Example 1: Find all Rust files recursively");
    println!("Pattern: **/*.rs\n");

    let call1 = create_tool_call(
        "call-1",
        "Glob",
        json!({
            "pattern": "**/*.rs"
        }),
    );

    match glob_tool.execute(&call1).await {
        Ok(result) => {
            if result.success {
                println!("Result:");
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution failed: {}", e),
    }

    println!("\n{}\n", "=".repeat(60));

    // Example 2: Find Cargo.toml files
    println!("Example 2: Find all Cargo.toml files");
    println!("Pattern: **/Cargo.toml\n");

    let call2 = create_tool_call(
        "call-2",
        "Glob",
        json!({
            "pattern": "**/Cargo.toml"
        }),
    );

    match glob_tool.execute(&call2).await {
        Ok(result) => {
            if result.success {
                println!("Result:");
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution failed: {}", e),
    }

    println!("\n{}\n", "=".repeat(60));

    // Example 3: Find files in a specific directory
    println!("Example 3: Find source files in crates/sage-core/src");
    println!("Pattern: **/*.rs");
    println!("Path: crates/sage-core/src\n");

    let call3 = create_tool_call(
        "call-3",
        "Glob",
        json!({
            "pattern": "**/*.rs",
            "path": "crates/sage-core/src"
        }),
    );

    match glob_tool.execute(&call3).await {
        Ok(result) => {
            if result.success {
                println!("Result:");
                let output = result.output.unwrap_or_default();
                // Show only first 20 lines for brevity
                let lines: Vec<&str> = output.lines().collect();
                for line in lines.iter().take(20) {
                    println!("{}", line);
                }
                if lines.len() > 20 {
                    println!("... and {} more files", lines.len() - 20);
                }
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution failed: {}", e),
    }

    println!("\n{}\n", "=".repeat(60));

    // Example 4: Find example files
    println!("Example 4: Find all example files");
    println!("Pattern: examples/*.rs\n");

    let call4 = create_tool_call(
        "call-4",
        "Glob",
        json!({
            "pattern": "examples/*.rs"
        }),
    );

    match glob_tool.execute(&call4).await {
        Ok(result) => {
            if result.success {
                println!("Result:");
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution failed: {}", e),
    }

    println!("\n{}\n", "=".repeat(60));

    // Example 5: Character class pattern
    println!("Example 5: Find markdown files starting with uppercase letters");
    println!("Pattern: [A-Z]*.md\n");

    let call5 = create_tool_call(
        "call-5",
        "Glob",
        json!({
            "pattern": "[A-Z]*.md"
        }),
    );

    match glob_tool.execute(&call5).await {
        Ok(result) => {
            if result.success {
                println!("Result:");
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution failed: {}", e),
    }

    Ok(())
}

/// Helper to create a tool call
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
