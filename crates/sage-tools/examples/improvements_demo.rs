//! Demonstration of the improved Sage Tools features
//!
//! This example showcases:
//! 1. Enhanced error handling with context and suggestions
//! 2. Tool monitoring and metrics
//! 3. Configuration system
//! 4. Improved task management

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::{
    config::{get_global_config, update_global_config},
    tools::{
        task_mgmt::{AddTasksTool, ViewTasklistTool, task_management::GLOBAL_TASK_LIST},
        utils::{
            enhanced_errors::helpers,
            monitoring::{get_monitoring_report, record_error, record_success},
        },
    },
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

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
    println!("🚀 Sage Tools Improvements Demo");
    println!("================================\n");

    // 1. Configuration System Demo
    println!("📋 1. Configuration System Demo");
    println!("---------------------------------");

    let config = get_global_config();
    println!(
        "Default working directory: {:?}",
        config.default_working_directory
    );
    println!(
        "Max execution time: {} seconds",
        config.max_execution_time_seconds
    );
    println!("Max output size: {} bytes", config.max_output_size_bytes);

    // Update configuration
    update_global_config(|config| {
        config.debug_logging = true;
        config.max_execution_time_seconds = 600; // 10 minutes
    });

    let updated_config = get_global_config();
    println!(
        "Updated max execution time: {} seconds",
        updated_config.max_execution_time_seconds
    );
    println!("Debug logging enabled: {}\n", updated_config.debug_logging);

    // 2. Enhanced Error Handling Demo
    println!("❌ 2. Enhanced Error Handling Demo");
    println!("-----------------------------------");

    // Simulate various error scenarios
    let file_error = helpers::file_not_found("/path/to/nonexistent/file.txt");
    println!(
        "File Not Found Error:\n{}",
        file_error.user_friendly_message()
    );

    let permission_error = helpers::permission_denied("write", "/etc/passwd");
    println!(
        "Permission Error:\n{}",
        permission_error.user_friendly_message()
    );

    let invalid_arg_error = helpers::invalid_argument("timeout", "invalid", "positive integer");
    println!(
        "Invalid Argument Error:\n{}",
        invalid_arg_error.user_friendly_message()
    );

    // 3. Tool Monitoring Demo
    println!("📊 3. Tool Monitoring Demo");
    println!("---------------------------");

    // Simulate some tool executions
    record_success("bash", Duration::from_millis(150));
    record_success("edit", Duration::from_millis(75));
    record_success("bash", Duration::from_millis(200));
    record_error(
        "json_edit",
        Duration::from_millis(50),
        "InvalidArguments".to_string(),
    );
    record_success("task_management", Duration::from_millis(25));
    record_error(
        "bash",
        Duration::from_millis(300),
        "ExecutionFailed".to_string(),
    );

    let report = get_monitoring_report();
    println!("{}", report.format());

    // 4. Task Management Demo (with existing functionality)
    println!("📝 4. Task Management Demo");
    println!("---------------------------");

    // Clear any existing tasks
    GLOBAL_TASK_LIST.clear_and_rebuild(vec![])?;

    // Add some tasks
    let add_tool = AddTasksTool::new();
    let call = create_tool_call(
        "demo-add",
        "AddTasks",
        json!({
            "tasks": [
                {
                    "name": "Implement Configuration System",
                    "description": "Add comprehensive configuration management",
                    "state": "COMPLETE"
                },
                {
                    "name": "Add Error Handling",
                    "description": "Implement enhanced error handling with suggestions",
                    "state": "COMPLETE"
                },
                {
                    "name": "Create Monitoring System",
                    "description": "Build tool execution monitoring and metrics",
                    "state": "COMPLETE"
                },
                {
                    "name": "Write Documentation",
                    "description": "Document all the new features and improvements",
                    "state": "IN_PROGRESS"
                }
            ]
        }),
    );

    let result = add_tool.execute(&call).await?;
    println!("Added tasks: {}", result.output.as_ref().unwrap());

    // View the task list
    let view_tool = ViewTasklistTool::new();
    let view_call = create_tool_call("demo-view", "ViewTasklist", json!({}));
    let view_result = view_tool.execute(&view_call).await?;
    println!(
        "\nCurrent Task List:\n{}",
        view_result.output.as_ref().unwrap()
    );

    // 5. JSON Error Logging Demo
    println!("📄 5. JSON Error Logging Demo");
    println!("------------------------------");

    let config_error = helpers::configuration_error("database.url", "invalid URL format");
    println!(
        "JSON Error Log:\n{}\n",
        serde_json::to_string_pretty(&config_error.to_json())?
    );

    // 6. Performance Metrics Summary
    println!("⚡ 6. Performance Summary");
    println!("-------------------------");

    let final_report = get_monitoring_report();
    println!("System uptime: {:?}", final_report.uptime);
    println!("Total tool executions: {}", final_report.total_executions);
    println!(
        "Overall success rate: {:.1}%",
        final_report.overall_success_rate * 100.0
    );

    if let Some(bash_metrics) = final_report
        .tool_metrics
        .iter()
        .find(|m| m.tool_name == "bash")
    {
        println!(
            "Bash tool average execution time: {:.1}ms",
            bash_metrics.average_execution_time_ms
        );
    }

    println!("\n🎉 Demo completed successfully!");
    println!("\n📈 Key Improvements Demonstrated:");
    println!("  ✅ Configuration system with runtime updates");
    println!("  ✅ Enhanced error handling with context and suggestions");
    println!("  ✅ Tool execution monitoring and metrics");
    println!("  ✅ JSON-structured error logging");
    println!("  ✅ Improved task management (existing feature)");
    println!("  ✅ User-friendly error messages");

    Ok(())
}
