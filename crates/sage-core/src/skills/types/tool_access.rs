//! Tool access control for skills

use serde::{Deserialize, Serialize};

/// Tool access control for skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolAccess {
    /// All tools available
    All,
    /// Only specific tools
    Only(Vec<String>),
    /// All except specific tools
    Except(Vec<String>),
    /// Read-only tools only
    ReadOnly,
}

impl ToolAccess {
    /// Check if a tool is allowed
    pub fn allows(&self, tool_name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(allowed) => allowed.iter().any(|t| t.eq_ignore_ascii_case(tool_name)),
            Self::Except(denied) => !denied.iter().any(|t| t.eq_ignore_ascii_case(tool_name)),
            Self::ReadOnly => {
                let read_only = ["Read", "Glob", "Grep", "WebFetch", "WebSearch"];
                read_only.iter().any(|t| t.eq_ignore_ascii_case(tool_name))
            }
        }
    }
}

impl Default for ToolAccess {
    fn default() -> Self {
        Self::All
    }
}
