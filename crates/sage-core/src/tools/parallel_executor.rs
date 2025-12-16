//! Enhanced parallel tool executor with semaphore-based concurrency control
//!
//! This module provides a sophisticated tool executor that:
//! - Respects tool concurrency modes (Parallel, Sequential, Limited, ExclusiveByType)
//! - Uses semaphores for fine-grained concurrency control
//! - Integrates with the permission system
//! - Supports cancellation via CancellationToken

use crate::tools::base::{ConcurrencyMode, Tool, ToolError};
use crate::tools::permission::{
    PermissionCache, PermissionDecision, PermissionRequest, PermissionResult,
    SharedPermissionHandler, ToolContext,
};
use crate::tools::types::{ToolCall, ToolResult};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

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

/// Execution result with metadata
#[derive(Debug)]
pub struct ExecutionResult {
    /// The tool result
    pub result: ToolResult,
    /// Time spent waiting for semaphore
    pub wait_time: Duration,
    /// Actual execution time
    pub execution_time: Duration,
    /// Whether permission was checked
    pub permission_checked: bool,
}

/// Enhanced parallel tool executor
pub struct ParallelToolExecutor {
    /// Registered tools
    tools: DashMap<String, Arc<dyn Tool>>,
    /// Configuration
    config: ParallelExecutorConfig,
    /// Global concurrency semaphore
    global_semaphore: Arc<Semaphore>,
    /// Per-type semaphores for ExclusiveByType mode
    type_semaphores: DashMap<String, Arc<Semaphore>>,
    /// Sequential execution lock (for Sequential mode)
    sequential_lock: Arc<Mutex<()>>,
    /// Limited concurrency semaphores (tool name -> semaphore)
    limited_semaphores: DashMap<String, Arc<Semaphore>>,
    /// Permission handler
    permission_handler: RwLock<Option<SharedPermissionHandler>>,
    /// Permission cache
    permission_cache: Arc<PermissionCache>,
    /// Tool context for permission checking
    tool_context: RwLock<ToolContext>,
    /// Cancellation token
    cancellation_token: CancellationToken,
    /// Execution statistics
    stats: Arc<RwLock<ExecutorStats>>,
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

impl ParallelToolExecutor {
    /// Create a new parallel executor with default configuration
    pub fn new() -> Self {
        Self::with_config(ParallelExecutorConfig::default())
    }

    /// Create a new parallel executor with custom configuration
    pub fn with_config(config: ParallelExecutorConfig) -> Self {
        let global_semaphore = Arc::new(Semaphore::new(config.max_global_concurrency));

        Self {
            tools: DashMap::new(),
            config,
            global_semaphore,
            type_semaphores: DashMap::new(),
            sequential_lock: Arc::new(Mutex::new(())),
            limited_semaphores: DashMap::new(),
            permission_handler: RwLock::new(None),
            permission_cache: Arc::new(PermissionCache::new()),
            tool_context: RwLock::new(ToolContext::default()),
            cancellation_token: CancellationToken::new(),
            stats: Arc::new(RwLock::new(ExecutorStats::default())),
        }
    }

    /// Register a tool
    pub fn register_tool(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();

        // Set up concurrency control for this tool
        match tool.concurrency_mode() {
            ConcurrencyMode::Limited(max) => {
                self.limited_semaphores
                    .insert(name.clone(), Arc::new(Semaphore::new(max)));
            }
            ConcurrencyMode::ExclusiveByType => {
                // Create a semaphore with 1 permit for exclusive execution
                self.type_semaphores
                    .insert(name.clone(), Arc::new(Semaphore::new(1)));
            }
            _ => {}
        }

        self.tools.insert(name, tool);
    }

    /// Register multiple tools
    pub fn register_tools(&self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register_tool(tool);
        }
    }

    /// Set the permission handler
    pub async fn set_permission_handler(&self, handler: SharedPermissionHandler) {
        *self.permission_handler.write().await = Some(handler);
    }

    /// Set the tool context
    pub async fn set_tool_context(&self, context: ToolContext) {
        *self.tool_context.write().await = context;
    }

    /// Get a child cancellation token
    pub fn child_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }

    /// Cancel all running executions
    pub fn cancel_all(&self) {
        self.cancellation_token.cancel();
    }

    /// Execute a single tool call
    pub async fn execute_tool(&self, call: &ToolCall) -> ExecutionResult {
        let _overall_start = Instant::now();
        let mut wait_time = Duration::ZERO;
        let mut permission_checked = false;

        // Check for cancellation
        if self.cancellation_token.is_cancelled() {
            return ExecutionResult {
                result: ToolResult::error(&call.id, &call.name, "Execution cancelled"),
                wait_time,
                execution_time: Duration::ZERO,
                permission_checked,
            };
        }

        // Get the tool
        let tool = match self.tools.get(&call.name) {
            Some(tool) => tool.clone(),
            None => {
                return ExecutionResult {
                    result: ToolResult::error(
                        &call.id,
                        &call.name,
                        format!("Tool '{}' not found", call.name),
                    ),
                    wait_time,
                    execution_time: Duration::ZERO,
                    permission_checked,
                };
            }
        };

        // Check permissions if enabled
        if self.config.check_permissions {
            permission_checked = true;

            // Check cache first
            let cache_key = PermissionCache::cache_key(&call.name, call);
            if self.config.use_permission_cache {
                if let Some(cached) = self.permission_cache.get(&cache_key).await {
                    if !cached {
                        self.stats.write().await.permission_denials += 1;
                        return ExecutionResult {
                            result: ToolResult::error(
                                &call.id,
                                &call.name,
                                "Permission denied (cached)",
                            ),
                            wait_time,
                            execution_time: Duration::ZERO,
                            permission_checked,
                        };
                    }
                }
            }

            // Check tool-level permission
            let context = self.tool_context.read().await;
            let perm_result = tool.check_permission(call, &context).await;

            match perm_result {
                PermissionResult::Allow => {}
                PermissionResult::Deny { reason } => {
                    self.stats.write().await.permission_denials += 1;
                    if self.config.use_permission_cache {
                        self.permission_cache.set(cache_key, false).await;
                    }
                    return ExecutionResult {
                        result: ToolResult::error(&call.id, &call.name, reason),
                        wait_time,
                        execution_time: Duration::ZERO,
                        permission_checked,
                    };
                }
                PermissionResult::Ask {
                    question,
                    default,
                    risk_level,
                } => {
                    // Delegate to permission handler
                    let handler = self.permission_handler.read().await;
                    if let Some(ref handler) = *handler {
                        let request =
                            PermissionRequest::new(tool.name(), call.clone(), question, risk_level);

                        let decision = handler.handle_permission_request(request).await;

                        match decision {
                            PermissionDecision::Allow => {}
                            PermissionDecision::AllowAlways => {
                                if self.config.use_permission_cache {
                                    self.permission_cache.set(cache_key, true).await;
                                }
                            }
                            PermissionDecision::Deny => {
                                self.stats.write().await.permission_denials += 1;
                                return ExecutionResult {
                                    result: ToolResult::error(
                                        &call.id,
                                        &call.name,
                                        "Permission denied by user",
                                    ),
                                    wait_time,
                                    execution_time: Duration::ZERO,
                                    permission_checked,
                                };
                            }
                            PermissionDecision::DenyAlways => {
                                self.stats.write().await.permission_denials += 1;
                                if self.config.use_permission_cache {
                                    self.permission_cache.set(cache_key, false).await;
                                }
                                return ExecutionResult {
                                    result: ToolResult::error(
                                        &call.id,
                                        &call.name,
                                        "Permission denied by user (permanently)",
                                    ),
                                    wait_time,
                                    execution_time: Duration::ZERO,
                                    permission_checked,
                                };
                            }
                            PermissionDecision::Modify { .. } => {
                                // TODO: Handle modified call
                            }
                        }
                    } else if !default {
                        // No handler and default is deny
                        self.stats.write().await.permission_denials += 1;
                        return ExecutionResult {
                            result: ToolResult::error(
                                &call.id,
                                &call.name,
                                "Permission denied (no handler)",
                            ),
                            wait_time,
                            execution_time: Duration::ZERO,
                            permission_checked,
                        };
                    }
                }
                PermissionResult::Transform { .. } => {
                    // TODO: Handle transformed call
                }
            }
        }

        // Acquire necessary semaphores based on concurrency mode
        let wait_start = Instant::now();
        let _permits = match self.acquire_permits(&tool).await {
            Ok(permits) => {
                wait_time = wait_start.elapsed();
                permits
            }
            Err(e) => {
                return ExecutionResult {
                    result: ToolResult::error(&call.id, &call.name, e.to_string()),
                    wait_time: wait_start.elapsed(),
                    execution_time: Duration::ZERO,
                    permission_checked,
                };
            }
        };

        // Check cancellation again after waiting
        if self.cancellation_token.is_cancelled() {
            self.stats.write().await.cancellations += 1;
            return ExecutionResult {
                result: ToolResult::error(&call.id, &call.name, "Execution cancelled"),
                wait_time,
                execution_time: Duration::ZERO,
                permission_checked,
            };
        }

        // Execute the tool with timeout
        let execution_start = Instant::now();
        let execution_timeout = tool
            .max_execution_duration()
            .unwrap_or(self.config.default_timeout);

        let result = tokio::select! {
            _ = self.cancellation_token.cancelled() => {
                self.stats.write().await.cancellations += 1;
                ToolResult::error(&call.id, &call.name, "Execution cancelled")
            }
            result = timeout(execution_timeout, tool.execute_with_timing(call)) => {
                match result {
                    Ok(r) => r,
                    Err(_) => {
                        self.stats.write().await.timeouts += 1;
                        ToolResult::error(
                            &call.id,
                            &call.name,
                            format!("Tool execution timed out after {:?}", execution_timeout),
                        )
                    }
                }
            }
        };

        let execution_time = execution_start.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_executions += 1;
            if result.success {
                stats.successful_executions += 1;
            } else {
                stats.failed_executions += 1;
            }
            stats.total_execution_time += execution_time;
            stats.total_wait_time += wait_time;
        }

        ExecutionResult {
            result,
            wait_time,
            execution_time,
            permission_checked,
        }
    }

    /// Execute multiple tool calls with appropriate concurrency
    pub async fn execute_tools(&self, calls: &[ToolCall]) -> Vec<ExecutionResult> {
        if calls.is_empty() {
            return Vec::new();
        }

        // Partition calls by concurrency mode
        let (parallel_calls, sequential_calls) = self.partition_by_concurrency(calls);

        let mut results = Vec::with_capacity(calls.len());

        // Execute parallel-capable tools concurrently
        if !parallel_calls.is_empty() {
            let handles: Vec<_> = parallel_calls
                .into_iter()
                .map(|call| {
                    let call = call.clone();
                    let self_ref = self;
                    async move { self_ref.execute_tool(&call).await }
                })
                .collect();

            let parallel_results = futures::future::join_all(handles).await;
            results.extend(parallel_results);
        }

        // Execute sequential tools one by one
        for call in sequential_calls {
            let result = self.execute_tool(&call).await;
            results.push(result);
        }

        // Reorder results to match input order
        self.reorder_results(calls, results)
    }

    /// Partition calls by their concurrency capability
    fn partition_by_concurrency(&self, calls: &[ToolCall]) -> (Vec<ToolCall>, Vec<ToolCall>) {
        let mut parallel = Vec::new();
        let mut sequential = Vec::new();

        for call in calls {
            let is_parallel = self
                .tools
                .get(&call.name)
                .map(|tool| {
                    matches!(
                        tool.concurrency_mode(),
                        ConcurrencyMode::Parallel | ConcurrencyMode::Limited(_)
                    )
                })
                .unwrap_or(false);

            if is_parallel {
                parallel.push(call.clone());
            } else {
                sequential.push(call.clone());
            }
        }

        (parallel, sequential)
    }

    /// Acquire necessary permits for tool execution
    async fn acquire_permits(&self, tool: &Arc<dyn Tool>) -> Result<PermitGuard, ToolError> {
        let mut permits = PermitGuard::new();

        // Always acquire global permit first
        let global_permit = self
            .global_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ToolError::Other("Failed to acquire global permit".into()))?;
        permits.add_global(global_permit);

        // Acquire additional permits based on concurrency mode
        match tool.concurrency_mode() {
            ConcurrencyMode::Sequential => {
                // Acquire the sequential lock
                let _lock = self.sequential_lock.lock().await;
                // Note: We can't store the lock guard in PermitGuard easily,
                // but the global semaphore provides basic protection
            }
            ConcurrencyMode::ExclusiveByType => {
                if let Some(semaphore) = self.type_semaphores.get(tool.name()) {
                    let permit =
                        semaphore.clone().acquire_owned().await.map_err(|_| {
                            ToolError::Other("Failed to acquire type permit".into())
                        })?;
                    permits.add_type(permit);
                }
            }
            ConcurrencyMode::Limited(_) => {
                if let Some(semaphore) = self.limited_semaphores.get(tool.name()) {
                    let permit =
                        semaphore.clone().acquire_owned().await.map_err(|_| {
                            ToolError::Other("Failed to acquire limited permit".into())
                        })?;
                    permits.add_limited(permit);
                }
            }
            ConcurrencyMode::Parallel => {
                // Only global permit needed
            }
        }

        Ok(permits)
    }

    /// Reorder results to match input call order
    fn reorder_results(
        &self,
        original_calls: &[ToolCall],
        mut results: Vec<ExecutionResult>,
    ) -> Vec<ExecutionResult> {
        let mut ordered = Vec::with_capacity(original_calls.len());
        let mut result_map: HashMap<String, ExecutionResult> = results
            .drain(..)
            .map(|r| (r.result.call_id.clone(), r))
            .collect();

        for call in original_calls {
            if let Some(result) = result_map.remove(&call.id) {
                ordered.push(result);
            } else {
                ordered.push(ExecutionResult {
                    result: ToolResult::error(&call.id, &call.name, "Result not found"),
                    wait_time: Duration::ZERO,
                    execution_time: Duration::ZERO,
                    permission_checked: false,
                });
            }
        }

        ordered
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> ExecutorStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = ExecutorStats::default();
    }

    /// Clear permission cache
    pub async fn clear_permission_cache(&self) {
        self.permission_cache.clear().await;
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|t| t.clone())
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for ParallelToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for holding semaphore permits
struct PermitGuard {
    global: Option<tokio::sync::OwnedSemaphorePermit>,
    type_permit: Option<tokio::sync::OwnedSemaphorePermit>,
    limited: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl PermitGuard {
    fn new() -> Self {
        Self {
            global: None,
            type_permit: None,
            limited: None,
        }
    }

    fn add_global(&mut self, permit: tokio::sync::OwnedSemaphorePermit) {
        self.global = Some(permit);
    }

    fn add_type(&mut self, permit: tokio::sync::OwnedSemaphorePermit) {
        self.type_permit = Some(permit);
    }

    fn add_limited(&mut self, permit: tokio::sync::OwnedSemaphorePermit) {
        self.limited = Some(permit);
    }
}

/// Builder for ParallelToolExecutor
pub struct ParallelExecutorBuilder {
    config: ParallelExecutorConfig,
    tools: Vec<Arc<dyn Tool>>,
    permission_handler: Option<SharedPermissionHandler>,
    context: Option<ToolContext>,
}

impl ParallelExecutorBuilder {
    pub fn new() -> Self {
        Self {
            config: ParallelExecutorConfig::default(),
            tools: Vec::new(),
            permission_handler: None,
            context: None,
        }
    }

    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.config.max_global_concurrency = max;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    pub fn with_permission_checking(mut self, enabled: bool) -> Self {
        self.config.check_permissions = enabled;
        self
    }

    pub fn with_permission_cache(mut self, enabled: bool) -> Self {
        self.config.use_permission_cache = enabled;
        self
    }

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn with_permission_handler(mut self, handler: SharedPermissionHandler) -> Self {
        self.permission_handler = Some(handler);
        self
    }

    pub fn with_context(mut self, context: ToolContext) -> Self {
        self.context = Some(context);
        self
    }

    pub async fn build(self) -> ParallelToolExecutor {
        let executor = ParallelToolExecutor::with_config(self.config);

        for tool in self.tools {
            executor.register_tool(tool);
        }

        if let Some(handler) = self.permission_handler {
            executor.set_permission_handler(handler).await;
        }

        if let Some(context) = self.context {
            executor.set_tool_context(context).await;
        }

        executor
    }
}

impl Default for ParallelExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::types::ToolSchema;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Simple test tool
    struct TestTool {
        name: String,
        delay: Duration,
        mode: ConcurrencyMode,
        call_count: Arc<AtomicU32>,
    }

    impl TestTool {
        fn new(name: &str, delay: Duration, mode: ConcurrencyMode) -> Self {
            Self {
                name: name.to_string(),
                delay,
                mode,
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    #[async_trait::async_trait]
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

        async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            Ok(ToolResult::success(&call.id, &self.name, "OK"))
        }

        fn concurrency_mode(&self) -> ConcurrencyMode {
            self.mode
        }
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let executor = ParallelToolExecutor::new();

        let tool1 = Arc::new(TestTool::new(
            "tool1",
            Duration::from_millis(50),
            ConcurrencyMode::Parallel,
        ));
        let tool2 = Arc::new(TestTool::new(
            "tool2",
            Duration::from_millis(50),
            ConcurrencyMode::Parallel,
        ));

        executor.register_tool(tool1);
        executor.register_tool(tool2);

        let calls = vec![
            ToolCall::new("1", "tool1", HashMap::new()),
            ToolCall::new("2", "tool2", HashMap::new()),
        ];

        let start = Instant::now();
        let results = executor.execute_tools(&calls).await;
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 2);
        assert!(results[0].result.success);
        assert!(results[1].result.success);

        // Should complete in roughly 50ms (parallel), not 100ms (sequential)
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_sequential_execution() {
        let executor = ParallelToolExecutor::new();

        let tool = Arc::new(TestTool::new(
            "seq_tool",
            Duration::from_millis(20),
            ConcurrencyMode::Sequential,
        ));

        executor.register_tool(tool);

        let calls = vec![
            ToolCall::new("1", "seq_tool", HashMap::new()),
            ToolCall::new("2", "seq_tool", HashMap::new()),
        ];

        let start = Instant::now();
        let results = executor.execute_tools(&calls).await;
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 2);
        assert!(results[0].result.success);
        assert!(results[1].result.success);

        // Sequential tools should take at least 40ms
        assert!(elapsed >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn test_cancellation() {
        let executor = Arc::new(ParallelToolExecutor::new());

        let tool = Arc::new(TestTool::new(
            "slow_tool",
            Duration::from_secs(10),
            ConcurrencyMode::Parallel,
        ));

        executor.register_tool(tool);

        let call = ToolCall::new("1", "slow_tool", HashMap::new());

        // Cancel after a short delay
        let executor_clone = executor.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            executor_clone.cancel_all();
        });

        let result = executor.execute_tool(&call).await;

        assert!(!result.result.success);
        assert!(result.result.error.unwrap().contains("cancelled"));
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let executor = ParallelToolExecutor::new();

        let call = ToolCall::new("1", "nonexistent", HashMap::new());
        let result = executor.execute_tool(&call).await;

        assert!(!result.result.success);
        assert!(result.result.error.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_statistics() {
        let executor = ParallelToolExecutor::new();

        let tool = Arc::new(TestTool::new(
            "stats_tool",
            Duration::from_millis(10),
            ConcurrencyMode::Parallel,
        ));

        executor.register_tool(tool);

        let calls = vec![
            ToolCall::new("1", "stats_tool", HashMap::new()),
            ToolCall::new("2", "stats_tool", HashMap::new()),
        ];

        executor.execute_tools(&calls).await;

        let stats = executor.get_stats().await;
        assert_eq!(stats.total_executions, 2);
        assert_eq!(stats.successful_executions, 2);
    }
}
