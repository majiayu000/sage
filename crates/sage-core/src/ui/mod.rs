//! User Interface components for Sage Agent

pub mod animation;
pub mod claude_style;
pub mod display;
pub mod enhanced_console;
pub mod markdown;
pub mod prompt;

pub use animation::AnimationManager;
pub use claude_style::{ClaudeStyleDisplay, ResponseFormatter, SimpleProgressIndicator};
pub use display::DisplayManager;
pub use enhanced_console::EnhancedConsole;
pub use markdown::{MarkdownRenderer, render_markdown, render_markdown_with_width};
pub use prompt::{
    PermissionChoice, PermissionDialogConfig, confirm, print_error, print_info, print_success,
    print_warning, show_permission_dialog,
};
