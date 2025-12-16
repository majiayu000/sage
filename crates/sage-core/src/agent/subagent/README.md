# Subagent Module

This module implements a subagent system for Sage, inspired by OpenClaude's agent patterns.

## Overview

The subagent system provides specialized agents with different capabilities, tool access, and system prompts. This allows for:

- **Focused agents** with limited tool access for specific tasks
- **Faster execution** using smaller models for simple tasks
- **Better prompts** tailored to specific use cases
- **Clear separation** of concerns between different types of tasks

## Components

### Types (`types.rs`)

Core type definitions:

- `AgentType`: Enumeration of agent types (GeneralPurpose, Explore, Plan, Custom)
- `ToolAccessControl`: Controls which tools an agent can access (All, Specific, None)
- `AgentDefinition`: Complete definition of an agent including name, description, tools, and prompt

### Built-in Agents (`builtin.rs`)

Three pre-defined agents matching OpenClaude patterns:

#### 1. General Purpose Agent
- **Tools**: All available tools
- **Model**: Default (inherits from parent)
- **Use case**: Complex multi-step tasks, code modifications, general engineering work

#### 2. Explore Agent
- **Tools**: glob, grep, read (read-only)
- **Model**: haiku (fast model)
- **Use case**: Quick codebase exploration, finding files, searching code

#### 3. Plan Agent
- **Tools**: All available tools
- **Model**: Default
- **Use case**: Architecture design, implementation planning, creating step-by-step plans

### Registry (`registry.rs`)

Manages available agent definitions and running agent instances:

- Register and retrieve agent definitions
- Track running agents with status and progress
- Support for cancellation tokens
- Thread-safe with Arc<RwLock<>>

## Usage

### Basic Usage

```rust
use sage_core::agent::{AgentRegistry, register_builtin_agents};

// Create registry and register built-in agents
let registry = AgentRegistry::new();
register_builtin_agents(&registry);

// Get an agent definition
let explore = registry.get(&AgentType::Explore).unwrap();
println!("Agent: {}", explore.name);
println!("Description: {}", explore.description);
```

### Custom Agents

```rust
use sage_core::agent::{AgentDefinition, ToolAccessControl};

// Create a custom agent
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

// Register it
registry.register(custom);
```

### Querying Agents

```rust
// By type
let agent = registry.get(&AgentType::Explore);

// By name (case-insensitive)
let agent = registry.get_by_name("explore");

// List all definitions
for agent in registry.list_definitions() {
    println!("{}: {}", agent.name, agent.description);
}
```

### Tool Access Control

```rust
let explore = registry.get(&AgentType::Explore).unwrap();

// Check if agent can use a tool
if explore.can_use_tool("read") {
    println!("Can read files");
}

if !explore.can_use_tool("write") {
    println!("Cannot write files (read-only)");
}
```

## Architecture

### Agent Lifecycle

1. **Definition**: Agent is defined with name, description, tools, and prompt
2. **Registration**: Agent definition is registered in the registry
3. **Instantiation**: When needed, a running agent is created from the definition
4. **Execution**: Agent executes tasks using its allowed tools
5. **Completion**: Agent status is updated and result is returned

### Design Decisions

- **Copy types**: AgentType is Copy for easy cloning and passing around
- **Thread-safe registry**: Uses Arc<RwLock<>> for concurrent access
- **Separate concerns**: Definitions (what agents are) vs. instances (running agents)
- **Tool access control**: Explicit allow-list prevents agents from using inappropriate tools

## Examples

See `examples/builtin_agents_demo.rs` for a comprehensive demonstration of:
- Creating and registering agents
- Querying agents by type and name
- Checking tool access
- Creating custom agents
- Inspecting system prompts

## Testing

The module includes comprehensive tests for:
- All three built-in agent definitions
- Agent registration and retrieval
- Tool access control
- Custom agent creation
- Type serialization and display

Run tests with:
```bash
cargo test --package sage-core --lib agent::subagent::builtin
```

## Future Enhancements

Potential improvements:
- Agent templating system for creating variations
- Agent composition (combining multiple agents)
- Dynamic tool filtering based on context
- Agent metrics and performance tracking
- Agent hot-reloading for development
