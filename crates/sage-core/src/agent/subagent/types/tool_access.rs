//! Tool access control for agents

use serde::{Deserialize, Serialize};
use std::fmt;

/// Tool access control for agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolAccessControl {
    /// Agent has access to all available tools
    All,
    /// Agent has access only to specific tools
    Specific(Vec<String>),
    /// No tool access
    None,
}

impl fmt::Display for ToolAccessControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolAccessControl::All => write!(f, "all_tools"),
            ToolAccessControl::Specific(tools) => {
                write!(f, "tools[{}]", tools.join(", "))
            }
            ToolAccessControl::None => write!(f, "no_tools"),
        }
    }
}

impl Default for ToolAccessControl {
    fn default() -> Self {
        ToolAccessControl::All
    }
}

impl ToolAccessControl {
    /// Check if a tool is allowed
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        match self {
            ToolAccessControl::All => true,
            ToolAccessControl::Specific(tools) => tools.iter().any(|t| t == tool_name),
            ToolAccessControl::None => false,
        }
    }

    /// Get the list of allowed tools (if specific)
    pub fn allowed_tools(&self) -> Option<&[String]> {
        match self {
            ToolAccessControl::Specific(tools) => Some(tools),
            _ => None,
        }
    }
}
