//! Command category and list types

use super::super::types::CommandSource;

/// Command category for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// System command (built-in)
    System,
    /// User-defined command (from Markdown files)
    User,
    /// MCP prompt command
    Mcp,
}

impl std::fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Mcp => write!(f, "mcp"),
        }
    }
}

impl From<&CommandSource> for CommandCategory {
    fn from(source: &CommandSource) -> Self {
        match source {
            CommandSource::Builtin => CommandCategory::System,
            CommandSource::Project | CommandSource::User => CommandCategory::User,
        }
    }
}

/// Information about a routed command
#[derive(Debug, Clone)]
pub struct RoutedCommand {
    /// Command name
    pub name: String,
    /// Command category
    pub category: CommandCategory,
    /// Command description
    pub description: Option<String>,
}

/// List of commands grouped by category
#[derive(Debug, Clone, Default)]
pub struct CommandList {
    /// System (built-in) commands
    pub system: Vec<RoutedCommand>,
    /// User-defined commands
    pub user: Vec<RoutedCommand>,
    /// MCP prompt commands
    pub mcp: Vec<RoutedCommand>,
}

impl CommandList {
    /// Total number of commands
    pub fn total(&self) -> usize {
        self.system.len() + self.user.len() + self.mcp.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// Get all commands as a flat list
    pub fn all(&self) -> Vec<&RoutedCommand> {
        self.system
            .iter()
            .chain(self.user.iter())
            .chain(self.mcp.iter())
            .collect()
    }
}
