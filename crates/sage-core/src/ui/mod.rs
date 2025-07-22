//! User Interface components for Sage Agent

pub mod animation;
pub mod claude_style;
pub mod display;
pub mod enhanced_console;
pub mod markdown;

pub use animation::AnimationManager;
pub use claude_style::{ClaudeStyleDisplay, ResponseFormatter, SimpleProgressIndicator};
pub use display::DisplayManager;
pub use enhanced_console::EnhancedConsole;
pub use markdown::{MarkdownRenderer, render_markdown, render_markdown_with_width};
