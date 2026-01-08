//! Extension tools for skill execution and slash commands

pub mod platform_tool_proxy;
pub mod skill;
pub mod slash_command;

// Re-export tools
pub use platform_tool_proxy::PlatformToolProxy;
pub use skill::SkillTool;
pub use slash_command::SlashCommandTool;
