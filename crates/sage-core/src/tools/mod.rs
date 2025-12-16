//! Tool system for Sage Agent

pub mod background_registry;
pub mod background_task;
pub mod base;
pub mod batch_executor;
pub mod executor;
pub mod parallel_executor;
pub mod permission;
pub mod registry;
pub mod tool_cache;
pub mod types;

pub use background_registry::{
    global_registry, BackgroundTaskRegistry, BackgroundTaskSummary, BACKGROUND_REGISTRY,
};
pub use background_task::{BackgroundShellTask, BackgroundTaskStatus};
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
pub use tool_cache::{
    CacheStats, CachedResult, SharedToolCache, ToolCache, ToolCacheConfig, ToolCacheKey,
    create_shared_cache,
};
pub use types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
