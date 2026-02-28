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
    AgentEvent, AgentEventDto, AppState, AppStateDto, EventAdapter, ExecutionPhase,
    ExecutionPhaseDto, InputState, InputStateDto, Message, MessageDto, Role, RoleDto,
    StreamingContentDto, ThinkingState, ToolExecution, ToolExecutionDto, ToolStatus, ToolStatusDto,
    UiMessageContent, UiMessageContentDto, UiSessionInfo, UiSessionInfoDto, UiToolResultDto,
};

pub use icons::{Icons, init_from_env as init_icons, is_nerd_fonts_enabled, set_nerd_fonts};
