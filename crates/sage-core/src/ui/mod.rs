//! User Interface components for Sage Agent

pub mod animation;
pub mod claude_style;
pub mod display;
pub mod enhanced_console;
pub mod icons;
pub mod markdown;
pub mod progress;
pub mod prompt;

pub use animation::{AnimationContext, AnimationManager};
pub use claude_style::{ClaudeStyleDisplay, ResponseFormatter, SimpleProgressIndicator};
pub use display::DisplayManager;
pub use icons::{Icons, init_from_env as init_icons, is_nerd_fonts_enabled, set_nerd_fonts};
pub use enhanced_console::EnhancedConsole;
pub use markdown::{MarkdownRenderer, render_markdown, render_markdown_with_width};
pub use progress::{ExecutionPhase, ProgressTracker, SubagentStatus, global_progress_tracker};
pub use prompt::{
    PermissionChoice, PermissionDialogConfig, confirm, print_error, print_info, print_success,
    print_warning, show_permission_dialog,
};
