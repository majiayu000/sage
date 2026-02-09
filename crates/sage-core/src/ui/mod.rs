//! User Interface components for Sage Agent
//!
//! This module provides the UI system with framework-agnostic abstractions.
//!
//! # Architecture
//!
//! - `traits/` - Framework-agnostic abstractions (EventSink, UiContext)
//! - `bridge/` - Event bridge between agent and UI
//! - `components/` - Reusable UI components
//! - `theme/` - Theming and styling
//! - `icons/` - Icon definitions

// === UI Abstractions ===
pub mod traits;

// === UI System ===
pub mod bridge;
pub mod components;
pub mod icons;
pub mod theme;

// === Re-exports: Traits ===
pub use traits::{EventSink, NoopEventSink, UiContext};

// === Re-exports: Bridge ===
pub use bridge::{
    AgentEvent, AppState, EventAdapter, ExecutionPhase, InputState, Message, MessageContent, Role,
    SessionState, ThinkingState, ToolExecution, ToolStatus,
};

// === Re-exports: Components ===
pub use components::{
    InputBox, MessageList, MessageView, Spinner, StatusBar, ThinkingIndicator, ToolExecutionView,
};
pub use icons::{Icons, init_from_env as init_icons, is_nerd_fonts_enabled, set_nerd_fonts};
pub use theme::{Colors, Icons as ThemeIcons, Styles};
