//! Demonstration of the built-in agents system
//!
//! This example shows how to:
//! - Create an agent registry
//! - Register built-in agents
//! - Query agents by type and name
//! - Inspect agent capabilities and tool access

use sage_core::agent::{
    AgentDefinition, AgentRegistry, AgentType, ToolAccessControl, register_builtin_agents,
};

fn main() {
    println!("=== Sage Built-in Agents Demo ===\n");

    // Create a new registry
    let registry = AgentRegistry::new();
    println!("Created empty agent registry");
    println!("Registry size: {}\n", registry.len());

    // Register all built-in agents
    register_builtin_agents(&registry);
    println!("Registered built-in agents");
    println!("Registry size: {}\n", registry.len());

    // List all available agents
    println!("Available Agents:");
    println!("{}", "=".repeat(60));
    for agent in registry.list_definitions() {
        print_agent_info(&agent);
        println!("{}", "-".repeat(60));
    }

    // Demonstrate querying agents
    println!("\n=== Querying Agents ===\n");

    // Query by type
    if let Some(explore) = registry.get(&AgentType::Explore) {
        println!("Found Explore agent by type:");
        println!("  Name: {}", explore.name);
        println!("  Model: {:?}", explore.model);
    }

    // Query by name (case-insensitive)
    if let Some(plan) = registry.get_by_name("plan") {
        println!("\nFound Plan agent by name (case-insensitive):");
        println!("  Type: {}", plan.agent_type);
        println!("  Description: {}", plan.description);
    }

    // Demonstrate tool access control
    println!("\n=== Tool Access Control ===\n");

    let general = registry.get(&AgentType::GeneralPurpose).unwrap();
    println!("General Purpose Agent - Tool Access:");
    check_tool_access(&general, &["glob", "grep", "read", "write", "bash", "edit"]);

    let explore = registry.get(&AgentType::Explore).unwrap();
    println!("\nExplore Agent - Tool Access:");
    check_tool_access(&explore, &["glob", "grep", "read", "write", "bash", "edit"]);

    // Show system prompts
    println!("\n=== System Prompts ===\n");
    for agent in registry.list_definitions() {
        println!("{}:", agent.name);
        println!("{}", "-".repeat(60));
        println!("{}", truncate_prompt(&agent.system_prompt, 200));
        println!("{}", "=".repeat(60));
        println!();
    }

    // Demonstrate creating custom agent
    println!("=== Custom Agent ===\n");
    let custom = AgentDefinition::custom(
        "Code Review".to_string(),
        "Agent specialized in reviewing code changes".to_string(),
        ToolAccessControl::Specific(vec![
            "glob".to_string(),
            "grep".to_string(),
            "read".to_string(),
            "git".to_string(),
        ]),
        "You are a code review agent...".to_string(),
    );
    println!("Created custom agent:");
    print_agent_info(&custom);

    println!("\n=== Demo Complete ===");
}

fn print_agent_info(agent: &AgentDefinition) {
    println!("Name: {}", agent.name);
    println!("Type: {}", agent.agent_type);
    println!("Description: {}", agent.description);
    println!("Model: {:?}", agent.model);

    print!("Tool Access: ");
    match &agent.available_tools {
        ToolAccessControl::All => println!("All tools"),
        ToolAccessControl::Specific(tools) => {
            println!("Specific tools ({})", tools.len());
            for tool in tools {
                println!("  - {}", tool);
            }
        }
        ToolAccessControl::None => println!("No tools"),
        ToolAccessControl::Inherited => println!("Inherited from parent"),
        ToolAccessControl::InheritedRestricted(tools) => {
            println!("Inherited (restricted to {} tools)", tools.len());
            for tool in tools {
                println!("  - {}", tool);
            }
        }
    }
}

fn check_tool_access(agent: &AgentDefinition, tools: &[&str]) {
    for tool in tools {
        let access = if agent.can_use_tool(tool) {
            "✓"
        } else {
            "✗"
        };
        println!("  {} {}", access, tool);
    }
}

fn truncate_prompt(prompt: &str, max_len: usize) -> String {
    if prompt.len() <= max_len {
        prompt.to_string()
    } else {
        format!("{}...", &prompt[..max_len])
    }
}
