//! Tool system for Sage Agent

pub mod background_registry;
pub mod background_task;
pub mod base;
pub mod executor;
pub mod names;
pub mod parallel_executor;
pub mod permission;
pub mod registry;
pub mod tool_cache;
pub mod types;

#[cfg(test)]
mod executor_tests;

pub use background_registry::{
    BACKGROUND_REGISTRY, BackgroundTaskRegistry, BackgroundTaskSummary, global_registry,
};
pub use background_task::{BackgroundShellTask, BackgroundTaskStatus};
pub use base::{
    ConcurrencyMode, FullTool, Tool, ToolConcurrency, ToolError, ToolMetadata, ToolPermission,
    ToolRenderer, ToolTiming, ToolValidator,
};
pub use executor::ToolExecutor;
pub use parallel_executor::{
    ExecutorStats, ParallelExecutorBuilder, ParallelExecutorConfig, ParallelToolExecutor,
    ToolExecutionResult,
};
pub use permission::{
    PermissionCache, PermissionDecision, PermissionHandler, PermissionPolicy, PermissionRequest,
    PolicyHandler, RiskLevel, SharedPermissionHandler, ToolContext, ToolPermissionResult,
};
pub use registry::ToolRegistry;
pub use tool_cache::{
    CachedResult, SharedToolCache, ToolCache, ToolCacheConfig, ToolCacheKey, ToolCacheStats,
    create_shared_cache,
};
pub use types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
