//! Core parallel tool executor implementation

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

use super::config::{ExecutionResult, ExecutorStats, ParallelExecutorConfig};

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

        match tool.concurrency_mode() {
            ConcurrencyMode::Limited(max) => {
                self.limited_semaphores
                    .insert(name.clone(), Arc::new(Semaphore::new(max)));
            }
            ConcurrencyMode::ExclusiveByType => {
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
            if let Some(result) = self.check_permission(call, &tool).await {
                return result;
            }
        }

        // Acquire necessary semaphores
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

        self.reorder_results(calls, results)
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

    // Private helper methods

    async fn check_permission(
        &self,
        call: &ToolCall,
        tool: &Arc<dyn Tool>,
    ) -> Option<ExecutionResult> {
        let cache_key = PermissionCache::cache_key(&call.name, call);

        if self.config.use_permission_cache {
            if let Some(cached) = self.permission_cache.get(&cache_key).await {
                if !cached {
                    self.stats.write().await.permission_denials += 1;
                    return Some(ExecutionResult {
                        result: ToolResult::error(&call.id, &call.name, "Permission denied (cached)"),
                        wait_time: Duration::ZERO,
                        execution_time: Duration::ZERO,
                        permission_checked: true,
                    });
                }
            }
        }

        let context = self.tool_context.read().await;
        let perm_result = tool.check_permission(call, &context).await;

        match perm_result {
            PermissionResult::Allow => None,
            PermissionResult::Deny { reason } => {
                self.stats.write().await.permission_denials += 1;
                if self.config.use_permission_cache {
                    self.permission_cache.set(cache_key, false).await;
                }
                Some(ExecutionResult {
                    result: ToolResult::error(&call.id, &call.name, reason),
                    wait_time: Duration::ZERO,
                    execution_time: Duration::ZERO,
                    permission_checked: true,
                })
            }
            PermissionResult::Ask { question, default, risk_level } => {
                self.handle_ask_permission(call, tool, &cache_key, question, default, risk_level).await
            }
            PermissionResult::Transform { .. } => None, // TODO: Handle transformed call
        }
    }

    async fn handle_ask_permission(
        &self,
        call: &ToolCall,
        tool: &Arc<dyn Tool>,
        cache_key: &str,
        question: String,
        default: bool,
        risk_level: crate::tools::permission::RiskLevel,
    ) -> Option<ExecutionResult> {
        let handler = self.permission_handler.read().await;

        if let Some(ref handler) = *handler {
            let request = PermissionRequest::new(tool.name(), call.clone(), question, risk_level);
            let decision = handler.handle_permission_request(request).await;

            match decision {
                PermissionDecision::Allow => None,
                PermissionDecision::AllowAlways => {
                    if self.config.use_permission_cache {
                        self.permission_cache.set(cache_key.to_string(), true).await;
                    }
                    None
                }
                PermissionDecision::Deny => {
                    self.stats.write().await.permission_denials += 1;
                    Some(ExecutionResult {
                        result: ToolResult::error(&call.id, &call.name, "Permission denied by user"),
                        wait_time: Duration::ZERO,
                        execution_time: Duration::ZERO,
                        permission_checked: true,
                    })
                }
                PermissionDecision::DenyAlways => {
                    self.stats.write().await.permission_denials += 1;
                    if self.config.use_permission_cache {
                        self.permission_cache.set(cache_key.to_string(), false).await;
                    }
                    Some(ExecutionResult {
                        result: ToolResult::error(&call.id, &call.name, "Permission denied (permanently)"),
                        wait_time: Duration::ZERO,
                        execution_time: Duration::ZERO,
                        permission_checked: true,
                    })
                }
                PermissionDecision::Modify { .. } => None, // TODO: Handle modified call
            }
        } else if !default {
            self.stats.write().await.permission_denials += 1;
            Some(ExecutionResult {
                result: ToolResult::error(&call.id, &call.name, "Permission denied (no handler)"),
                wait_time: Duration::ZERO,
                execution_time: Duration::ZERO,
                permission_checked: true,
            })
        } else {
            None
        }
    }

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

    async fn acquire_permits(&self, tool: &Arc<dyn Tool>) -> Result<PermitGuard, ToolError> {
        let mut permits = PermitGuard::new();

        let global_permit = self
            .global_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ToolError::Other("Failed to acquire global permit".into()))?;
        permits.add_global(global_permit);

        match tool.concurrency_mode() {
            ConcurrencyMode::Sequential => {
                let _lock = self.sequential_lock.lock().await;
            }
            ConcurrencyMode::ExclusiveByType => {
                if let Some(semaphore) = self.type_semaphores.get(tool.name()) {
                    let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                        ToolError::Other("Failed to acquire type permit".into())
                    })?;
                    permits.add_type(permit);
                }
            }
            ConcurrencyMode::Limited(_) => {
                if let Some(semaphore) = self.limited_semaphores.get(tool.name()) {
                    let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                        ToolError::Other("Failed to acquire limited permit".into())
                    })?;
                    permits.add_limited(permit);
                }
            }
            ConcurrencyMode::Parallel => {}
        }

        Ok(permits)
    }

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
