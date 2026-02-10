//! UI Bridge - Agent and UI communication layer
//!
//! This module provides the decoupled bridge between Agent execution
//! and UI rendering, enabling:
//! - Clear separation of concerns
//! - Easy UI framework replacement
//! - Testable UI state management

pub mod adapter;
pub mod events;
pub mod state;

#[allow(deprecated)]
pub use adapter::{
    EventAdapter, emit_event, global_adapter, set_global_adapter, set_refresh_callback,
};
pub use events::AgentEvent;
pub use state::{
    AppState, ExecutionPhase, InputState, Message, UiMessageContent, MessageMetadata, Role,
    UiSessionInfo, StreamingContent, ThinkingState, ToolExecution, UiToolResult, ToolStatus, UiConfig,
};

// Re-export tokio watch types for subscribers
pub use tokio::sync::watch::Receiver as StateReceiver;
