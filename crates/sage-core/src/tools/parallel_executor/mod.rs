//! Enhanced parallel tool executor with semaphore-based concurrency control
//!
//! This module provides a sophisticated tool executor that:
//! - Respects tool concurrency modes (Parallel, Sequential, Limited, ExclusiveByType)
//! - Uses semaphores for fine-grained concurrency control
//! - Integrates with the permission system
//! - Supports cancellation via CancellationToken

mod builder;
mod config;
mod executor;

pub use builder::ParallelExecutorBuilder;
pub use config::{ExecutionResult, ExecutorStats, ParallelExecutorConfig};
pub use executor::ParallelToolExecutor;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::{ConcurrencyMode, Tool, ToolError};
    use crate::tools::permission::{PermissionResult, ToolContext};
    use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // Simple test tool
    struct TestTool {
        name: String,
        delay: Duration,
        call_count: AtomicU32,
        concurrency_mode: ConcurrencyMode,
    }

    impl TestTool {
        fn new(name: &str, delay: Duration, concurrency: ConcurrencyMode) -> Self {
            Self {
                name: name.to_string(),
                delay,
                call_count: AtomicU32::new(0),
                concurrency_mode: concurrency,
            }
        }
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "Test tool"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name.clone(), "Test tool".to_string(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            Ok(ToolResult::success("test_id", &self.name, "done"))
        }

        fn concurrency_mode(&self) -> ConcurrencyMode {
            self.concurrency_mode.clone()
        }

        async fn check_permission(&self, _call: &ToolCall, _context: &ToolContext) -> PermissionResult {
            PermissionResult::Allow
        }
    }

    #[tokio::test]
    async fn test_basic_execution() {
        let executor = ParallelToolExecutor::new();
        let tool = Arc::new(TestTool::new("test", Duration::from_millis(10), ConcurrencyMode::Parallel));
        executor.register_tool(tool);

        let call = ToolCall::new("1", "test", HashMap::new());

        let result = executor.execute_tool(&call).await;
        assert!(result.result.success);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let executor = ParallelToolExecutor::new();
        let tool = Arc::new(TestTool::new("test", Duration::from_millis(50), ConcurrencyMode::Parallel));
        executor.register_tool(tool.clone());

        let calls: Vec<_> = (0..5)
            .map(|i| ToolCall::new(i.to_string(), "test".to_string(), HashMap::new()))
            .collect();

        let start = std::time::Instant::now();
        let results = executor.execute_tools(&calls).await;
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.result.success));
        // Should complete faster than sequential (5 * 50ms = 250ms)
        assert!(elapsed < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let executor = ParallelToolExecutor::new();

        let call = ToolCall::new("1", "nonexistent", HashMap::new());

        let result = executor.execute_tool(&call).await;
        assert!(!result.result.success);
        assert!(result.result.error.as_ref().map_or(false, |e| e.contains("not found")));
    }

    #[tokio::test]
    async fn test_builder() {
        let tool = Arc::new(TestTool::new("test", Duration::from_millis(10), ConcurrencyMode::Parallel));

        let executor = ParallelExecutorBuilder::new()
            .with_max_concurrency(8)
            .with_timeout(Duration::from_secs(60))
            .with_permission_checking(false)
            .with_tool(tool)
            .build()
            .await;

        assert!(!executor.tool_names().is_empty());
    }
}
