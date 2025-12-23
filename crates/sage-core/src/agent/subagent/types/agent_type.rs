//! Agent type enumeration and implementations

use serde::{Deserialize, Serialize};
use std::fmt;

/// Agent type enumeration defining different agent specializations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// General purpose agent with all tools available
    GeneralPurpose,
    /// Fast exploration agent with read-only tools (Glob, Grep, Read)
    Explore,
    /// Architecture planning agent with all tools
    Plan,
    /// Custom agent type
    Custom,
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for AgentType {
    fn default() -> Self {
        AgentType::GeneralPurpose
    }
}

impl AgentType {
    /// Get the string identifier for this agent type
    pub fn as_str(&self) -> &str {
        match self {
            AgentType::GeneralPurpose => "general_purpose",
            AgentType::Explore => "explore",
            AgentType::Plan => "plan",
            AgentType::Custom => "custom",
        }
    }
}
