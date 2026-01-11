//! User Interface components for Sage Agent
//!
//! This module provides both legacy UI components and the new
//! rnk-based declarative UI system.

// === New UI System (rnk-based) ===
pub mod bridge;
pub mod components;
pub mod theme;

// === Legacy UI (to be removed) ===
pub mod animation;
pub mod claude_style;
pub mod display;
pub mod enhanced_console;
pub mod icons;
pub mod markdown;
pub mod progress;
pub mod prompt;

// === Re-exports: New UI ===
pub use bridge::{
    AgentEvent, AppState, EventAdapter, ExecutionPhase, InputState, Message, MessageContent,
    Role, SessionState, ThinkingState, ToolExecution, ToolStatus,
};
pub use components::{
    InputBox, MessageList, MessageView, Spinner, StatusBar, ThinkingIndicator, ToolExecutionView,
};
pub use theme::{Colors, Icons as ThemeIcons, Styles};

// === Re-exports: Legacy UI (deprecated, will be removed) ===
pub use animation::{AnimationContext, AnimationManager};
pub use claude_style::{ClaudeStyleDisplay, ResponseFormatter, SimpleProgressIndicator};
pub use display::DisplayManager;
pub use enhanced_console::EnhancedConsole;
pub use icons::{init_from_env as init_icons, is_nerd_fonts_enabled, set_nerd_fonts, Icons};
pub use markdown::{render_markdown, render_markdown_with_width, MarkdownRenderer};
pub use progress::{global_progress_tracker, ExecutionPhase as LegacyExecutionPhase, ProgressTracker, SubagentStatus};
pub use prompt::{
    confirm, print_error, print_info, print_success, print_warning, show_permission_dialog,
    PermissionChoice, PermissionDialogConfig,
};
