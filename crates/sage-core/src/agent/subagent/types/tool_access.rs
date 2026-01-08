//! Tool access control for agents
//!
//! Defines how sub-agents access tools, including inheritance from parent agents.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Tool access control for agents
///
/// Controls which tools a sub-agent can use. Supports inheritance from parent
/// agents, explicit tool lists, and restricted access patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolAccessControl {
    /// Agent has access to all available tools
    All,
    /// Agent has access only to specific tools
    Specific(Vec<String>),
    /// No tool access
    None,
    /// Inherit tools from parent agent
    ///
    /// When used, the sub-agent will have access to the same tools as its parent.
    /// This is the recommended default for sub-agents that need to perform
    /// similar operations as their parent.
    Inherited,
    /// Inherit from parent but restrict to specific tools
    ///
    /// Only allows tools that are both in the parent's tool set AND in this list.
    /// Useful for creating specialized sub-agents with limited capabilities.
    InheritedRestricted(Vec<String>),
}

impl fmt::Display for ToolAccessControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolAccessControl::All => write!(f, "all_tools"),
            ToolAccessControl::Specific(tools) => {
                write!(f, "tools[{}]", tools.join(", "))
            }
            ToolAccessControl::None => write!(f, "no_tools"),
            ToolAccessControl::Inherited => write!(f, "inherited"),
            ToolAccessControl::InheritedRestricted(tools) => {
                write!(f, "inherited_restricted[{}]", tools.join(", "))
            }
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
    ///
    /// For `Inherited` and `InheritedRestricted`, this method checks against
    /// the restriction list. To properly resolve inheritance, use `resolve_allows_tool`
    /// with the parent's tool list.
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        match self {
            ToolAccessControl::All => true,
            ToolAccessControl::Specific(tools) => tools.iter().any(|t| t == tool_name),
            ToolAccessControl::None => false,
            // For Inherited, we assume all tools are allowed
            // The actual filtering happens during resolution
            ToolAccessControl::Inherited => true,
            // For InheritedRestricted, check the restriction list
            ToolAccessControl::InheritedRestricted(tools) => tools.iter().any(|t| t == tool_name),
        }
    }

    /// Check if a tool is allowed, considering parent tools for inheritance
    ///
    /// # Arguments
    /// * `tool_name` - The name of the tool to check
    /// * `parent_tools` - Names of tools available to the parent agent
    ///
    /// # Returns
    /// `true` if the tool is allowed given the parent context
    pub fn resolve_allows_tool(&self, tool_name: &str, parent_tools: Option<&[String]>) -> bool {
        match self {
            ToolAccessControl::All => true,
            ToolAccessControl::Specific(tools) => tools.iter().any(|t| t == tool_name),
            ToolAccessControl::None => false,
            ToolAccessControl::Inherited => {
                // If inherited, check if tool exists in parent's tools
                match parent_tools {
                    Some(tools) => tools.iter().any(|t| t == tool_name),
                    None => true, // No parent context, allow all
                }
            }
            ToolAccessControl::InheritedRestricted(restricted) => {
                // Must be in both restriction list AND parent tools
                let in_restriction = restricted.iter().any(|t| t == tool_name);
                let in_parent = match parent_tools {
                    Some(tools) => tools.iter().any(|t| t == tool_name),
                    None => true, // No parent context, only check restriction
                };
                in_restriction && in_parent
            }
        }
    }

    /// Get the list of allowed tools (if specific)
    pub fn allowed_tools(&self) -> Option<&[String]> {
        match self {
            ToolAccessControl::Specific(tools) => Some(tools),
            ToolAccessControl::InheritedRestricted(tools) => Some(tools),
            _ => None,
        }
    }

    /// Check if this access control requires parent context for resolution
    pub fn requires_parent_context(&self) -> bool {
        matches!(
            self,
            ToolAccessControl::Inherited | ToolAccessControl::InheritedRestricted(_)
        )
    }

    /// Create an inherited restricted configuration
    pub fn inherited_restricted(tools: Vec<String>) -> Self {
        ToolAccessControl::InheritedRestricted(tools)
    }
}
