//! Example demonstrating the Grep tool functionality

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::GrepTool;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Grep Tool Demo ===\n");

    // Get current working directory
    let cwd = env::current_dir()?;
    println!("Working directory: {}\n", cwd.display());

    // Create the Grep tool
    let grep_tool = GrepTool::with_working_directory(cwd);

    // Example 1: Search for pattern with files_with_matches mode (default)
    println!("Example 1: Search for 'async fn' in Rust files");
    println!("================================================");
    let mut args = HashMap::new();
    args.insert(
        "pattern".to_string(),
        serde_json::json!("async fn"),
    );
    args.insert(
        "type".to_string(),
        serde_json::json!("rust"),
    );
    args.insert(
        "output_mode".to_string(),
        serde_json::json!("files_with_matches"),
    );
    args.insert(
        "head_limit".to_string(),
        serde_json::json!(5),
    );

    let call = ToolCall {
        id: "example-1".to_string(),
        name: "Grep".to_string(),
        arguments: args,
        call_id: None,
    };

    match grep_tool.execute(&call).await {
        Ok(result) => {
            if result.success {
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution error: {}", e),
    }

    println!("\n");

    // Example 2: Search with content mode and line numbers
    println!("Example 2: Search for 'TODO' with content and line numbers");
    println!("===========================================================");
    let mut args = HashMap::new();
    args.insert(
        "pattern".to_string(),
        serde_json::json!("TODO"),
    );
    args.insert(
        "type".to_string(),
        serde_json::json!("rust"),
    );
    args.insert(
        "output_mode".to_string(),
        serde_json::json!("content"),
    );
    args.insert(
        "-n".to_string(),
        serde_json::json!(true),
    );
    args.insert(
        "-A".to_string(),
        serde_json::json!(1),
    );
    args.insert(
        "head_limit".to_string(),
        serde_json::json!(3),
    );

    let call = ToolCall {
        id: "example-2".to_string(),
        name: "Grep".to_string(),
        arguments: args,
        call_id: None,
    };

    match grep_tool.execute(&call).await {
        Ok(result) => {
            if result.success {
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution error: {}", e),
    }

    println!("\n");

    // Example 3: Case-insensitive search
    println!("Example 3: Case-insensitive search for 'error'");
    println!("===============================================");
    let mut args = HashMap::new();
    args.insert(
        "pattern".to_string(),
        serde_json::json!("error"),
    );
    args.insert(
        "-i".to_string(),
        serde_json::json!(true),
    );
    args.insert(
        "output_mode".to_string(),
        serde_json::json!("count"),
    );
    args.insert(
        "type".to_string(),
        serde_json::json!("rust"),
    );
    args.insert(
        "head_limit".to_string(),
        serde_json::json!(10),
    );

    let call = ToolCall {
        id: "example-3".to_string(),
        name: "Grep".to_string(),
        arguments: args,
        call_id: None,
    };

    match grep_tool.execute(&call).await {
        Ok(result) => {
            if result.success {
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution error: {}", e),
    }

    println!("\n");

    // Example 4: Search with glob pattern
    println!("Example 4: Search in Cargo.toml files");
    println!("======================================");
    let mut args = HashMap::new();
    args.insert(
        "pattern".to_string(),
        serde_json::json!("version"),
    );
    args.insert(
        "glob".to_string(),
        serde_json::json!("Cargo.toml"),
    );
    args.insert(
        "output_mode".to_string(),
        serde_json::json!("files_with_matches"),
    );

    let call = ToolCall {
        id: "example-4".to_string(),
        name: "Grep".to_string(),
        arguments: args,
        call_id: None,
    };

    match grep_tool.execute(&call).await {
        Ok(result) => {
            if result.success {
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution error: {}", e),
    }

    println!("\n");

    // Example 5: Regex pattern search
    println!("Example 5: Regex pattern search for function definitions");
    println!("=========================================================");
    let mut args = HashMap::new();
    args.insert(
        "pattern".to_string(),
        serde_json::json!(r"pub fn \w+\("),
    );
    args.insert(
        "type".to_string(),
        serde_json::json!("rust"),
    );
    args.insert(
        "output_mode".to_string(),
        serde_json::json!("content"),
    );
    args.insert(
        "-n".to_string(),
        serde_json::json!(true),
    );
    args.insert(
        "head_limit".to_string(),
        serde_json::json!(5),
    );

    let call = ToolCall {
        id: "example-5".to_string(),
        name: "Grep".to_string(),
        arguments: args,
        call_id: None,
    };

    match grep_tool.execute(&call).await {
        Ok(result) => {
            if result.success {
                println!("{}", result.output.unwrap_or_default());
            } else {
                println!("Error: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Execution error: {}", e),
    }

    println!("\n=== Demo Complete ===");

    Ok(())
}
