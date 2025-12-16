//! Batch Tool Executor - Claude Code style concurrent tool execution
//!
//! This module implements intelligent batch tool execution with automatic
//! parallelization and dependency resolution.

use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Batch execution strategy
#[derive(Debug, Clone)]
pub enum BatchStrategy {
    /// Execute all tools in parallel (Claude Code default)
    Parallel,
    /// Execute tools sequentially
    Sequential,
    /// Smart execution based on tool dependencies and characteristics
    Smart,
}

/// Execution statistics for monitoring
#[derive(Debug, Clone)]
pub struct BatchExecutionStats {
    pub total_tools: usize,
    pub parallel_tools: usize,
    pub sequential_tools: usize,
    pub failed_tools: usize,
    pub total_duration: Duration,
    pub fastest_tool: Option<(String, Duration)>,
    pub slowest_tool: Option<(String, Duration)>,
}

/// Claude Code style batch tool executor
pub struct BatchToolExecutor {
    tools: HashMap<String, Arc<dyn Tool>>,
    strategy: BatchStrategy,
    max_parallel_tools: usize,
    default_timeout: Duration,

    // Performance tracking
    execution_stats: Option<BatchExecutionStats>,
}

impl BatchToolExecutor {
    /// Create a new batch executor with Claude Code defaults
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            strategy: BatchStrategy::Smart, // Smart by default like Claude Code
            max_parallel_tools: 8,          // Reasonable parallelism limit
            default_timeout: Duration::from_secs(300),
            execution_stats: None,
        }
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Register multiple tools at once
    pub fn register_tools(&mut self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register_tool(tool);
        }
    }

    /// Get tool schemas for LLM
    pub fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        self.tools.values().map(|tool| tool.schema()).collect()
    }

    /// Execute a batch of tool calls with intelligent concurrency
    pub async fn execute_batch(&mut self, calls: &[ToolCall]) -> Vec<ToolResult> {
        if calls.is_empty() {
            return Vec::new();
        }

        let _start_time = Instant::now();

        match self.strategy {
            BatchStrategy::Parallel => self.execute_parallel_batch(calls).await,
            BatchStrategy::Sequential => self.execute_sequential_batch(calls).await,
            BatchStrategy::Smart => self.execute_smart_batch(calls).await,
        }
    }

    /// Execute tools in parallel (Claude Code style)
    async fn execute_parallel_batch(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        // Group tools by their parallel execution capability
        let (parallel_calls, sequential_calls): (Vec<_>, Vec<_>) = calls.iter().partition(|call| {
            self.tools
                .get(&call.name)
                .map(|tool| tool.supports_parallel_execution())
                .unwrap_or(false)
        });

        let mut results = Vec::new();

        // Execute parallel tools concurrently
        if !parallel_calls.is_empty() {
            let parallel_results = self.execute_concurrent_tools(&parallel_calls).await;
            results.extend(parallel_results);
        }

        // Execute sequential tools one by one
        for call in sequential_calls {
            let result = self.execute_single_tool(call).await;
            results.push(result);
        }

        // Reorder results to match input order
        self.reorder_results(calls, results)
    }

    /// Execute tools one by one
    async fn execute_sequential_batch(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        let mut results = Vec::new();

        for call in calls {
            let result = self.execute_single_tool(call).await;
            results.push(result);
        }

        results
    }

    /// Smart execution based on tool characteristics (Claude Code approach)
    async fn execute_smart_batch(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        // Analyze tool calls for optimal execution strategy
        let analysis = self.analyze_tool_calls(calls);

        if analysis.should_parallelize {
            self.execute_parallel_batch(calls).await
        } else {
            self.execute_sequential_batch(calls).await
        }
    }

    /// Execute multiple tools concurrently
    async fn execute_concurrent_tools(&self, calls: &[&ToolCall]) -> Vec<ToolResult> {
        // Limit concurrency to prevent resource exhaustion
        let chunks: Vec<_> = calls.chunks(self.max_parallel_tools).collect();

        let mut all_results = Vec::new();

        for chunk in chunks {
            let chunk_futures: Vec<_> = chunk
                .iter()
                .map(|call| self.execute_single_tool(call))
                .collect();

            let chunk_results = futures::future::join_all(chunk_futures).await;
            all_results.extend(chunk_results);
        }

        all_results
    }

    /// Execute a single tool with timeout
    async fn execute_single_tool(&self, call: &ToolCall) -> ToolResult {
        let tool = match self.tools.get(&call.name) {
            Some(tool) => tool,
            None => {
                return ToolResult::error(
                    &call.id,
                    &call.name,
                    format!("Tool '{}' not found", call.name),
                );
            }
        };

        // Use tool-specific timeout or default
        let execution_timeout = tool
            .max_execution_time()
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Execute with timeout
        match timeout(execution_timeout, tool.execute_with_timing(call)).await {
            Ok(result) => result,
            Err(_) => ToolResult::error(
                &call.id,
                &call.name,
                format!("Tool execution timed out after {:?}", execution_timeout),
            ),
        }
    }

    /// Analyze tool calls to determine optimal execution strategy
    fn analyze_tool_calls(&self, calls: &[ToolCall]) -> BatchAnalysis {
        let mut analysis = BatchAnalysis::default();

        // Check if tools can run in parallel
        let parallel_capable = calls
            .iter()
            .filter(|call| {
                self.tools
                    .get(&call.name)
                    .map(|tool| tool.supports_parallel_execution())
                    .unwrap_or(false)
            })
            .count();

        // Heuristics for smart execution
        analysis.should_parallelize =
            parallel_capable > 1 && calls.len() <= self.max_parallel_tools;
        analysis.parallel_tools = parallel_capable;
        analysis.sequential_tools = calls.len() - parallel_capable;
        analysis.has_expensive_tools = calls.iter().any(|call| self.is_expensive_tool(&call.name));

        analysis
    }

    /// Check if a tool is considered expensive (I/O bound, network calls, etc.)
    fn is_expensive_tool(&self, tool_name: &str) -> bool {
        match tool_name {
            "bash" | "str_replace_based_edit_tool" | "codebase-retrieval" => true,
            _ => false,
        }
    }

    /// Reorder results to match the original call order
    fn reorder_results(
        &self,
        original_calls: &[ToolCall],
        mut results: Vec<ToolResult>,
    ) -> Vec<ToolResult> {
        let mut ordered_results = Vec::with_capacity(original_calls.len());

        for call in original_calls {
            if let Some(pos) = results.iter().position(|r| r.call_id == call.id) {
                ordered_results.push(results.remove(pos));
            } else {
                // Create error result for missing tool result
                ordered_results.push(ToolResult::error(
                    &call.id,
                    &call.name,
                    "Tool result not found in batch execution".to_string(),
                ));
            }
        }

        ordered_results
    }

    /// Set execution strategy
    pub fn set_strategy(&mut self, strategy: BatchStrategy) {
        self.strategy = strategy;
    }

    /// Set maximum parallel tools
    pub fn set_max_parallel_tools(&mut self, max: usize) {
        self.max_parallel_tools = max.max(1); // At least 1
    }

    /// Set default timeout
    pub fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> Option<&BatchExecutionStats> {
        self.execution_stats.as_ref()
    }
}

impl Default for BatchToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Analysis result for batch execution
#[derive(Debug, Default)]
struct BatchAnalysis {
    should_parallelize: bool,
    parallel_tools: usize,
    sequential_tools: usize,
    has_expensive_tools: bool,
}

/// Builder for batch tool executor
pub struct BatchExecutorBuilder {
    strategy: BatchStrategy,
    max_parallel_tools: usize,
    default_timeout: Duration,
    tools: Vec<Arc<dyn Tool>>,
}

impl BatchExecutorBuilder {
    pub fn new() -> Self {
        Self {
            strategy: BatchStrategy::Smart,
            max_parallel_tools: 8,
            default_timeout: Duration::from_secs(300),
            tools: Vec::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: BatchStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_max_parallel_tools(mut self, max: usize) -> Self {
        self.max_parallel_tools = max;
        self
    }

    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
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

    pub fn build(self) -> BatchToolExecutor {
        let mut executor = BatchToolExecutor::new();
        executor.set_strategy(self.strategy);
        executor.set_max_parallel_tools(self.max_parallel_tools);
        executor.set_default_timeout(self.default_timeout);

        for tool in self.tools {
            executor.register_tool(tool);
        }

        executor
    }
}

impl Default for BatchExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
