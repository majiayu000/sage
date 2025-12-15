//! Example demonstrating the Skill and SlashCommand tools
//!
//! This example shows how to use the extension tools for executing
//! specialized skills and custom slash commands.

use sage_tools::tools::extensions::{SkillTool, SlashCommandTool};
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Sage Extension Tools Demo ===\n");

    // Demonstrate SkillTool
    demo_skill_tool().await?;
    println!("\n");

    // Demonstrate SlashCommandTool
    demo_slash_command_tool().await?;

    Ok(())
}

async fn demo_skill_tool() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Skill Tool Demo ---");

    let skill_tool = SkillTool::new();

    // Display tool information
    println!("Tool name: {}", skill_tool.name());
    println!("Description: {}", skill_tool.description());
    println!();

    // Example 1: Execute brainstorming skill
    println!("Example 1: Execute 'brainstorming' skill");
    let mut args = HashMap::new();
    args.insert("skill".to_string(), serde_json::Value::String("brainstorming".to_string()));

    let call = ToolCall {
        id: "call-1".to_string(),
        name: "skill".to_string(),
        arguments: args,
        call_id: None,
    };

    let result = skill_tool.execute(&call).await?;
    println!("Success: {}", result.success);
    if let Some(output) = result.output {
        println!("Output: {}", output);
    }
    println!();

    // Example 2: Execute comprehensive-testing skill
    println!("Example 2: Execute 'comprehensive-testing' skill");
    let mut args = HashMap::new();
    args.insert("skill".to_string(), serde_json::Value::String("comprehensive-testing".to_string()));

    let call = ToolCall {
        id: "call-2".to_string(),
        name: "skill".to_string(),
        arguments: args,
        call_id: None,
    };

    let result = skill_tool.execute(&call).await?;
    println!("Success: {}", result.success);
    if let Some(output) = result.output {
        println!("Output: {}", output);
    }

    Ok(())
}

async fn demo_slash_command_tool() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Slash Command Tool Demo ---");

    let slash_tool = SlashCommandTool::new();

    // Display tool information
    println!("Tool name: {}", slash_tool.name());
    println!("Description: {}", slash_tool.description());
    println!();

    // Example 1: Execute simple command
    println!("Example 1: Execute '/test' command");
    let mut args = HashMap::new();
    args.insert("command".to_string(), serde_json::Value::String("/test".to_string()));

    let call = ToolCall {
        id: "call-3".to_string(),
        name: "slash_command".to_string(),
        arguments: args,
        call_id: None,
    };

    let result = slash_tool.execute(&call).await?;
    println!("Success: {}", result.success);
    if let Some(output) = result.output {
        println!("Output:\n{}", output);
    }
    println!();

    // Example 2: Execute command with arguments
    println!("Example 2: Execute '/review-pr 123' command");
    let mut args = HashMap::new();
    args.insert("command".to_string(), serde_json::Value::String("/review-pr 123".to_string()));

    let call = ToolCall {
        id: "call-4".to_string(),
        name: "slash_command".to_string(),
        arguments: args,
        call_id: None,
    };

    let result = slash_tool.execute(&call).await?;
    println!("Success: {}", result.success);
    if let Some(output) = result.output {
        println!("Output:\n{}", output);
    }

    Ok(())
}
