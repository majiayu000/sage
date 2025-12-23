//! Planning Mode Demonstration
//!
//! This example demonstrates the planning mode tools which allow the agent
//! to switch between planning and execution modes.

use sage_tools::tools::{EnterPlanModeTool, ExitPlanModeTool};
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() {
    println!("Testing Planning Mode Tools\n");
    println!("{}", "=".repeat(60));

    // Test EnterPlanModeTool
    let enter_tool = EnterPlanModeTool::new();
    println!("\n1. EnterPlanModeTool");
    println!("   Name: {}", enter_tool.name());
    println!("   Description: {}", enter_tool.description());

    let mut args = HashMap::new();
    let call = ToolCall {
        id: "test-1".to_string(),
        name: "enter_plan_mode".to_string(),
        arguments: args.clone(),
        call_id: None,
    };

    match enter_tool.execute(&call).await {
        Ok(result) => {
            println!("\n   Execution Result:");
            if let Some(output) = result.output {
                println!("{}", output);
            }
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    // Test ExitPlanModeTool
    let exit_tool = ExitPlanModeTool::new();
    println!("\n\n2. ExitPlanModeTool (without swarm)");
    println!("   Name: {}", exit_tool.name());
    println!("   Description: {}", exit_tool.description());

    args.clear();
    let call = ToolCall {
        id: "test-2".to_string(),
        name: "exit_plan_mode".to_string(),
        arguments: args.clone(),
        call_id: None,
    };

    match exit_tool.execute(&call).await {
        Ok(result) => {
            println!("\n   Execution Result:");
            if let Some(output) = result.output {
                println!("{}", output);
            }
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    // Test ExitPlanModeTool with swarm
    println!("\n\n3. ExitPlanModeTool (with swarm)");
    args.clear();
    args.insert("launchSwarm".to_string(), json!(true));
    args.insert("teammateCount".to_string(), json!(5));

    let call = ToolCall {
        id: "test-3".to_string(),
        name: "exit_plan_mode".to_string(),
        arguments: args,
        call_id: None,
    };

    match exit_tool.execute(&call).await {
        Ok(result) => {
            println!("\n   Execution Result:");
            if let Some(output) = result.output {
                println!("{}", output);
            }
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    println!("\n{}", "=".repeat(60));
    println!("All tests completed successfully!");
}
