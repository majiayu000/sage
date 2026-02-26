//! User Interface abstractions for Sage Agent
//!
//! This module provides the UI system with framework-agnostic abstractions.
//!
//! # Architecture
//!
//! - `traits/` - Framework-agnostic abstractions (EventSink, UiContext)
//! - `bridge/` - Event bridge between agent and UI
//! - `icons/` - Icon definitions

// === UI Abstractions ===
pub mod traits;

// === UI System ===
pub mod bridge;
pub mod icons;

// === Re-exports: Traits ===
pub use traits::{EventSink, NoopEventSink, UiContext};

// === Re-exports: Bridge ===
pub use bridge::{
    AgentEvent, AppState, EventAdapter, ExecutionPhase, InputState, Message, Role, ThinkingState,
    ToolExecution, ToolStatus, UiMessageContent, UiSessionInfo,
};

pub use icons::{Icons, init_from_env as init_icons, is_nerd_fonts_enabled, set_nerd_fonts};
