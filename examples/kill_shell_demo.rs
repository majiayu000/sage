//! Demonstration of the KillShell tool
//!
//! This example shows how to use the KillShell tool to terminate background shell processes.

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::KillShellTool;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== KillShell Tool Demo ===\n");

    // Create the KillShell tool
    let tool = KillShellTool::new();

    // Display tool information
    println!("Tool Name: {}", tool.name());
    println!("Description: {}\n", tool.description());

    // Example 1: Try to kill a non-existent shell
    println!("Example 1: Attempting to kill a non-existent shell");
    println!("---------------------------------------------------");

    let mut arguments = HashMap::new();
    arguments.insert(
        "shell_id".to_string(),
        serde_json::Value::String("nonexistent_shell".to_string()),
    );

    let call = ToolCall {
        id: "call_1".to_string(),
        name: tool.name().to_string(),
        arguments,
        call_id: None,
    };

    match tool.execute(&call).await {
        Ok(result) => {
            println!("Result: {:?}", result);
        }
        Err(e) => {
            println!("Expected error: {}", e);
        }
    }

    println!();

    // Example 2: Validation with invalid shell ID
    println!("Example 2: Validation with invalid shell ID format");
    println!("---------------------------------------------------");

    let mut arguments = HashMap::new();
    arguments.insert(
        "shell_id".to_string(),
        serde_json::Value::String("invalid shell!".to_string()),
    );

    let call = ToolCall {
        id: "call_2".to_string(),
        name: tool.name().to_string(),
        arguments,
        call_id: None,
    };

    match tool.validate(&call) {
        Ok(_) => println!("Validation passed (unexpected)"),
        Err(e) => println!("Expected validation error: {}", e),
    }

    println!();

    // Example 3: Show tool schema
    println!("Example 3: Tool Schema");
    println!("----------------------");

    let schema = tool.schema();
    println!("Schema: {}", serde_json::to_string_pretty(&schema)?);

    println!();

    // Example 4: Demonstrate with a real process (Unix only)
    #[cfg(unix)]
    {
        use std::process::Command;

        println!("Example 4: Kill a real background process (Unix only)");
        println!("-----------------------------------------------------");

        // Start a background sleep process
        let mut child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("Failed to spawn sleep process");

        let pid = child.id();
        println!("Started background process with PID: {}", pid);

        // Register the shell
        sage_tools::tools::process::kill_shell::register_shell(
            "demo_shell".to_string(),
            pid,
        ).await;

        println!("Registered shell as 'demo_shell'");

        // Now kill it
        let mut arguments = HashMap::new();
        arguments.insert(
            "shell_id".to_string(),
            serde_json::Value::String("demo_shell".to_string()),
        );

        let call = ToolCall {
            id: "call_3".to_string(),
            name: tool.name().to_string(),
            arguments,
            call_id: None,
        };

        match tool.execute(&call).await {
            Ok(result) => {
                println!("\nResult:");
                println!("  Success: {}", result.success);
                println!("  Output: {}", result.output.unwrap_or_default());
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }

        // Clean up: ensure child is killed
        let _ = child.kill();
        let _ = child.wait();
    }

    println!("\n=== Demo Complete ===");

    Ok(())
}
