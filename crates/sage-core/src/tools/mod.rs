//! Tool system for Sage Agent

pub mod base;
pub mod batch_executor;
pub mod executor;
pub mod parallel_executor;
pub mod permission;
pub mod registry;
pub mod types;

pub use base::{ConcurrencyMode, Tool, ToolError};
pub use batch_executor::{BatchExecutionStats, BatchStrategy, BatchToolExecutor};
pub use executor::ToolExecutor;
pub use parallel_executor::{
    ExecutionResult, ExecutorStats, ParallelExecutorBuilder, ParallelExecutorConfig,
    ParallelToolExecutor,
};
pub use permission::{
    PermissionCache, PermissionDecision, PermissionHandler, PermissionPolicy, PermissionRequest,
    PermissionResult, PolicyHandler, RiskLevel, SharedPermissionHandler, ToolContext,
};
pub use registry::ToolRegistry;
pub use types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
