//! User Interface components for Sage Agent

pub mod animation;
pub mod display;
pub mod markdown;
pub mod enhanced_console;

pub use animation::AnimationManager;
pub use display::DisplayManager;
pub use markdown::{MarkdownRenderer, render_markdown, render_markdown_with_width};
pub use enhanced_console::EnhancedConsole;
