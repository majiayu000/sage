//! Subagent system for specialized agent roles
//!
//! This module provides a framework for creating and managing specialized agents
//! with different capabilities, tool access, and system prompts. Inspired by
//! OpenClaude's agent patterns.
//!
//! # Built-in Agents
//!
//! - **General Purpose**: Full tool access for complex multi-step tasks
//! - **Explore**: Fast, read-only agent for codebase exploration
//! - **Plan**: Architecture and planning agent for design work
//!
//! # Example
//!
//! ```rust
//! use sage_core::agent::subagent::{AgentRegistry, AgentType, register_builtin_agents};
//!
//! let registry = AgentRegistry::new();
//! register_builtin_agents(&registry);
//!
//! // Get the explore agent for fast codebase search
//! let explore = registry.get(&AgentType::Explore).unwrap();
//! println!("Agent: {}", explore.name);
//! println!("Description: {}", explore.description);
//! ```

pub mod builtin;
// pub mod executor; // Temporarily disabled - has compilation errors
pub mod registry;
pub mod runner;
pub mod types;

pub use builtin::{
    explore_agent, general_purpose_agent, get_builtin_agents, plan_agent, register_builtin_agents,
};
// pub use executor::{ExecutorMessage, SubAgentExecutor};
pub use registry::AgentRegistry;
pub use runner::{
    SubAgentRunner, execute_subagent, get_global_runner, init_global_runner,
    init_global_runner_from_config, update_global_runner_tools,
};
pub use types::{
    AgentDefinition, AgentProgress, AgentStatus, AgentType, ExecutionMetadata, RunningAgent,
    SubAgentConfig, SubAgentResult, Thoroughness, ToolAccessControl,
};
