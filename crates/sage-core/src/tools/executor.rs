//! Tool execution engine

use crate::error::{SageError, SageResult};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Tool executor that manages and executes tools
pub struct ToolExecutor {
    tools: HashMap<String, Arc<dyn Tool>>,
    max_execution_time: Duration,
    allow_parallel_execution: bool,
    // TODO: Add tool dependency management
    // - Track tool dependencies and execution order
    // - Implement dependency resolution algorithm
    // - Support conditional tool execution based on dependencies

    // TODO: Add resource management
    // - Implement resource pooling for expensive tools
    // - Add memory and CPU usage limits per tool
    // - Support tool execution queuing and prioritization

    // TODO: Add tool security and sandboxing
    // - Implement tool permission system
    // - Add execution environment isolation
    // - Support tool capability restrictions
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            max_execution_time: Duration::from_secs(300), // 5 minutes default
            allow_parallel_execution: true,
        }
    }

    /// Create a tool executor with custom settings
    pub fn with_settings(max_execution_time: Duration, allow_parallel_execution: bool) -> Self {
        Self {
            tools: HashMap::new(),
            max_execution_time,
            allow_parallel_execution,
        }
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Register multiple tools
    pub fn register_tools(&mut self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register_tool(tool);
        }
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Get all registered tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Execute a single tool call
    pub async fn execute_tool(&self, call: &ToolCall) -> ToolResult {
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

        // Determine execution timeout
        let execution_timeout = tool
            .max_execution_time()
            .map(Duration::from_secs)
            .unwrap_or(self.max_execution_time);

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

    /// Execute multiple tool calls
    pub async fn execute_tools(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        if calls.is_empty() {
            return Vec::new();
        }

        if self.allow_parallel_execution && calls.len() > 1 {
            self.execute_tools_parallel(calls).await
        } else {
            self.execute_tools_sequential(calls).await
        }
    }

    /// Execute tools sequentially
    async fn execute_tools_sequential(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        let mut results = Vec::with_capacity(calls.len());

        for call in calls {
            let result = self.execute_tool(call).await;
            results.push(result);
        }

        results
    }

    /// Execute tools in parallel
    async fn execute_tools_parallel(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        // Check if all tools support parallel execution
        let can_run_parallel = calls.iter().all(|call| {
            self.tools
                .get(&call.name)
                .map(|tool| tool.supports_parallel_execution())
                .unwrap_or(false)
        });

        if !can_run_parallel {
            return self.execute_tools_sequential(calls).await;
        }

        // Execute in parallel
        let futures: Vec<_> = calls.iter().map(|call| self.execute_tool(call)).collect();

        futures::future::join_all(futures).await
    }

    /// Validate tool calls before execution
    pub fn validate_calls(&self, calls: &[ToolCall]) -> SageResult<()> {
        for call in calls {
            // Check if tool exists
            let tool = self
                .tools
                .get(&call.name)
                .ok_or_else(|| SageError::tool(&call.name, "Tool not found"))?;

            // Validate the call
            tool.validate(call)
                .map_err(|e| SageError::tool(&call.name, e.to_string()))?;
        }

        Ok(())
    }

    /// Get tool schemas for all registered tools
    pub fn get_tool_schemas(&self) -> Vec<crate::tools::types::ToolSchema> {
        self.tools.values().map(|tool| tool.schema()).collect()
    }

    /// Get tool schemas for specific tools
    pub fn get_schemas_for_tools(
        &self,
        tool_names: &[String],
    ) -> Vec<crate::tools::types::ToolSchema> {
        tool_names
            .iter()
            .filter_map(|name| self.tools.get(name))
            .map(|tool| tool.schema())
            .collect()
    }

    /// Set maximum execution time
    pub fn set_max_execution_time(&mut self, duration: Duration) {
        self.max_execution_time = duration;
    }

    /// Set parallel execution setting
    pub fn set_allow_parallel_execution(&mut self, allow: bool) {
        self.allow_parallel_execution = allow;
    }

    /// Get execution statistics
    pub fn get_statistics(&self) -> ExecutorStatistics {
        ExecutorStatistics {
            total_tools: self.tools.len(),
            tool_names: self.tool_names(),
            max_execution_time: self.max_execution_time,
            allow_parallel_execution: self.allow_parallel_execution,
        }
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the tool executor
#[derive(Debug, Clone)]
pub struct ExecutorStatistics {
    /// Total number of registered tools
    pub total_tools: usize,
    /// Names of all registered tools
    pub tool_names: Vec<String>,
    /// Maximum execution time setting
    pub max_execution_time: Duration,
    /// Whether parallel execution is allowed
    pub allow_parallel_execution: bool,
}

/// Builder for tool executor
pub struct ToolExecutorBuilder {
    tools: Vec<Arc<dyn Tool>>,
    max_execution_time: Duration,
    allow_parallel_execution: bool,
}

impl ToolExecutorBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            max_execution_time: Duration::from_secs(300),
            allow_parallel_execution: true,
        }
    }

    /// Add a tool
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set maximum execution time
    pub fn with_max_execution_time(mut self, duration: Duration) -> Self {
        self.max_execution_time = duration;
        self
    }

    /// Set parallel execution setting
    pub fn with_parallel_execution(mut self, allow: bool) -> Self {
        self.allow_parallel_execution = allow;
        self
    }

    /// Build the tool executor
    pub fn build(self) -> ToolExecutor {
        let mut executor =
            ToolExecutor::with_settings(self.max_execution_time, self.allow_parallel_execution);

        for tool in self.tools {
            executor.register_tool(tool);
        }

        executor
    }
}

impl Default for ToolExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
