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

pub use adapter::{emit_event, global_adapter, set_global_adapter, EventAdapter};
pub use events::AgentEvent;
pub use state::{
    AppState, ExecutionPhase, InputState, Message, MessageContent, MessageMetadata, Role,
    SessionState, StreamingContent, ThinkingState, ToolExecution, ToolResult, ToolStatus,
    UiConfig,
};

// Re-export tokio watch types for subscribers
pub use tokio::sync::watch::Receiver as StateReceiver;
