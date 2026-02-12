//! Base trait and types for tools
//!
//! This module provides the core abstractions for building tools that agents
//! can use to interact with the environment. It includes:
//!
//! - [`Tool`] - The main trait that all tools must implement
//! - [`ToolError`] - Error types for tool operations
//! - [`ConcurrencyMode`] - Control over parallel tool execution
//! - [`FileSystemTool`] - Helper trait for file-based tools
//! - [`CommandTool`] - Helper trait for command execution tools
//!
//! # Examples
//!
//! Basic tool implementation:
//!
//! ```no_run
//! use sage_core::tools::{Tool, ToolSchema};
//! use sage_core::tools::base::ToolError;
//! use sage_core::tools::types::{ToolCall, ToolResult};
//! use async_trait::async_trait;
//!
//! struct MyTool;
//!
//! #[async_trait]
//! impl Tool for MyTool {
//!     fn name(&self) -> &str { "my_tool" }
//!     fn description(&self) -> &str { "A custom tool" }
//!     fn schema(&self) -> ToolSchema {
//!         ToolSchema::new(self.name(), self.description(), vec![])
//!     }
//!     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
//!         Ok(ToolResult::success(&call.id, self.name(), "done"))
//!     }
//! }
//! ```

// Module declarations
pub mod command_tool;
pub mod concurrency;
pub mod error;
pub mod filesystem_tool;
pub mod tool_macro;
pub mod tool_trait;

// Re-exports
pub use command_tool::CommandTool;
pub use concurrency::ConcurrencyMode;
pub use error::ToolError;
pub use filesystem_tool::FileSystemTool;
pub use tool_trait::{
    FullTool, Tool, ToolConcurrency, ToolMetadata, ToolPermission, ToolRenderer, ToolTiming,
    ToolValidator,
};
