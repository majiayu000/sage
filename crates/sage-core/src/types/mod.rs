//! Shared types for the Sage Agent system (Layer 0)
//!
//! This module contains types shared across multiple sage-core modules.
//! **Layer 0 principle**: `types/` has zero dependencies on other sage-core modules,
//! only depending on external crates (serde, chrono, thiserror, etc.).

mod common;
pub mod message;
pub mod provider;
pub mod todo;
pub mod tool;
pub mod tool_error;

pub use common::{Id, TaskMetadata, TokenUsage};
pub use message::MessageRole;
pub use provider::{LlmProvider, TimeoutConfig};
pub use todo::{TodoItem, TodoStatus};
pub use tool::{ToolCall, ToolParameter, ToolResult, ToolSchema};
pub use tool_error::ToolError;
