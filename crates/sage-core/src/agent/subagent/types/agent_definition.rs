//! Agent definition containing configuration and metadata

use super::{AgentType, ToolAccessControl};
use std::fmt;

/// Agent definition containing configuration and metadata
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Type of agent
    pub agent_type: AgentType,
    /// Human-readable name
    pub name: String,
    /// Description of agent's purpose
    pub description: String,
    /// Tools available to this agent
    pub available_tools: ToolAccessControl,
    /// Optional model override
    pub model: Option<String>,
    /// System prompt for this agent
    pub system_prompt: String,
}

impl fmt::Display for AgentDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AgentDefinition({}: {}, tools: {})",
            self.name, self.agent_type, self.available_tools
        )
    }
}

impl AgentDefinition {
    /// Create a new custom agent definition
    pub fn custom(
        name: String,
        description: String,
        available_tools: ToolAccessControl,
        system_prompt: String,
    ) -> Self {
        Self {
            agent_type: AgentType::Custom,
            name,
            description,
            available_tools,
            model: None,
            system_prompt,
        }
    }

    /// Get the agent's identifier (used for registry lookups)
    pub fn id(&self) -> String {
        self.agent_type.as_str().to_string()
    }

    /// Check if this agent can use a specific tool
    pub fn can_use_tool(&self, tool_name: &str) -> bool {
        self.available_tools.allows_tool(tool_name)
    }
}
