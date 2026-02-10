//! Configuration and statistics for parallel tool executor

use std::time::Duration;

/// Configuration for the parallel executor
#[derive(Debug, Clone)]
pub struct ParallelExecutorConfig {
    /// Maximum number of tools that can run in parallel globally
    pub max_global_concurrency: usize,
    /// Default timeout for tool execution
    pub default_timeout: Duration,
    /// Whether to check permissions before execution
    pub check_permissions: bool,
    /// Whether to use the permission cache
    pub use_permission_cache: bool,
}

impl Default for ParallelExecutorConfig {
    fn default() -> Self {
        Self {
            max_global_concurrency: 16,
            default_timeout: Duration::from_secs(300),
            check_permissions: true,
            use_permission_cache: true,
        }
    }
}

/// Executor statistics
#[derive(Debug, Default, Clone)]
pub struct ExecutorStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub permission_denials: u64,
    pub timeouts: u64,
    pub cancellations: u64,
    pub total_execution_time: Duration,
    pub total_wait_time: Duration,
}

/// Execution result with metadata
#[derive(Debug)]
pub struct ToolExecutionResult {
    /// The tool result
    pub result: crate::tools::types::ToolResult,
    /// Time spent waiting for semaphore
    pub wait_time: Duration,
    /// Actual execution time
    pub execution_time: Duration,
    /// Whether permission was checked
    pub permission_checked: bool,
}
