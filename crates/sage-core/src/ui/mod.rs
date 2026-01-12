//! User Interface components for Sage Agent
//!
//! This module provides the rnk-based declarative UI system.

// === New UI System (rnk-based) ===
pub mod bridge;
pub mod components;
pub mod icons;
pub mod theme;

// === Re-exports: New UI ===
pub use bridge::{
    AgentEvent, AppState, EventAdapter, ExecutionPhase, InputState, Message, MessageContent,
    Role, SessionState, ThinkingState, ToolExecution, ToolStatus,
};
pub use components::{
    InputBox, MessageList, MessageView, Spinner, StatusBar, ThinkingIndicator, ToolExecutionView,
};
pub use icons::{init_from_env as init_icons, is_nerd_fonts_enabled, set_nerd_fonts, Icons};
pub use theme::{Colors, Icons as ThemeIcons, Styles};
