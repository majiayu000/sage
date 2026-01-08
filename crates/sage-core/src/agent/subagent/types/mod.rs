//! Core types for sub-agent orchestration system
//!
//! This module defines the fundamental types used in the sub-agent system,
//! including agent definitions, execution states, and result types.
//!
//! # Key Types
//!
//! - [`AgentDefinition`] - Definition of an agent including system prompt and tool access
//! - [`SubAgentConfig`] - Configuration for spawning a sub-agent
//! - [`WorkingDirectoryConfig`] - How sub-agents inherit/configure working directories
//! - [`ToolAccessControl`] - How sub-agents access tools (including inheritance)

mod agent_definition;
mod agent_type;
mod config;
mod progress;
mod running_agent;
mod status;
mod thoroughness;
mod tool_access;
mod working_directory;

#[cfg(test)]
mod tests;

// Re-export all public types
pub use agent_definition::AgentDefinition;
pub use agent_type::AgentType;
pub use config::SubAgentConfig;
pub use progress::{AgentProgress, ExecutionMetadata};
pub use running_agent::RunningAgent;
pub use status::{AgentStatus, SubAgentResult};
pub use thoroughness::Thoroughness;
pub use tool_access::ToolAccessControl;
pub use working_directory::WorkingDirectoryConfig;
