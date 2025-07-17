//! Demonstration of the task management tools
//! 
//! This example shows how to use the task management tools to:
//! 1. View an empty task list
//! 2. Add multiple tasks
//! 3. Update task states
//! 4. View the updated task list

use sage_tools::task_management::{ViewTasklistTool, AddTasksTool, UpdateTasksTool};
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use serde_json::json;
use std::collections::HashMap;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Task Management Tools Demo");
    println!("==============================\n");

    // Initialize tools
    let view_tool = ViewTasklistTool::new();
    let add_tool = AddTasksTool::new();
    let update_tool = UpdateTasksTool::new();

    // Step 1: View empty task list
    println!("📋 Step 1: Viewing empty task list");
    let call = create_tool_call("demo-1", "view_tasklist", json!({}));
    let result = view_tool.execute(&call).await?;
    println!("Result: {}\n", result.output.as_ref().unwrap());

    // Step 2: Add some tasks
    println!("➕ Step 2: Adding tasks");
    let call = create_tool_call("demo-2", "add_tasks", json!({
        "tasks": [
            {
                "name": "Setup Development Environment",
                "description": "Install Rust, configure IDE, and set up project structure",
                "state": "NOT_STARTED"
            },
            {
                "name": "Implement Core Features",
                "description": "Build the main functionality of the application",
                "state": "NOT_STARTED"
            },
            {
                "name": "Write Unit Tests",
                "description": "Create comprehensive test suite for all components",
                "state": "NOT_STARTED"
            },
            {
                "name": "Documentation",
                "description": "Write user documentation and API docs",
                "state": "NOT_STARTED"
            }
        ]
    }));
    let result = add_tool.execute(&call).await?;
    println!("Result: {}\n", result.output.as_ref().unwrap());

    // Step 3: View the populated task list
    println!("📋 Step 3: Viewing populated task list");
    let call = create_tool_call("demo-3", "view_tasklist", json!({}));
    let result = view_tool.execute(&call).await?;
    println!("Result:\n{}\n", result.output.as_ref().unwrap());

    // Step 4: Start working on first task
    println!("🔄 Step 4: Starting work on first task");
    
    // Get the first task ID from the task list
    let task_ids = {
        use sage_tools::task_management::GLOBAL_TASK_LIST;
        GLOBAL_TASK_LIST.get_root_task_ids()
    };

    if !task_ids.is_empty() {
        let call = create_tool_call("demo-4", "update_tasks", json!({
            "tasks": [{
                "task_id": task_ids[0],
                "state": "IN_PROGRESS"
            }]
        }));
        let result = update_tool.execute(&call).await?;
        println!("Result: {}\n", result.output.as_ref().unwrap());
    }

    // Step 5: Complete first task and start second
    println!("✅ Step 5: Completing first task and starting second");
    if task_ids.len() >= 2 {
        let call = create_tool_call("demo-5", "update_tasks", json!({
            "tasks": [
                {
                    "task_id": task_ids[0],
                    "state": "COMPLETE"
                },
                {
                    "task_id": task_ids[1],
                    "state": "IN_PROGRESS"
                }
            ]
        }));
        let result = update_tool.execute(&call).await?;
        println!("Result: {}\n", result.output.as_ref().unwrap());
    }

    // Step 6: View final state
    println!("📋 Step 6: Viewing final task list state");
    let call = create_tool_call("demo-6", "view_tasklist", json!({}));
    let result = view_tool.execute(&call).await?;
    println!("Final Result:\n{}", result.output.as_ref().unwrap());

    println!("\n🎉 Demo completed successfully!");
    println!("\nTask State Legend:");
    println!("[ ] = NOT_STARTED");
    println!("[/] = IN_PROGRESS");
    println!("[-] = CANCELLED");
    println!("[x] = COMPLETE");

    Ok(())
}
