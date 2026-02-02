//! Extension tools for skill execution, slash commands, and tool discovery

pub mod platform_tool_proxy;
pub mod skill;
pub mod slash_command;
pub mod tool_search;

// Re-export tools
pub use platform_tool_proxy::PlatformToolProxy;
pub use skill::SkillTool;
pub use slash_command::SlashCommandTool;
pub use tool_search::{DeferredToolInfo, DeferredToolRegistry, ToolSearchResult, ToolSearchTool};
