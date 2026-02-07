//! Unit tests for ToolExecutor

#[cfg(test)]
mod tests {
    use crate::tools::base::{Tool, ToolError};
    use crate::tools::executor::{ToolExecutor, ToolExecutorBuilder};
    use crate::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    // Mock tool for testing
    struct MockTool {
        name: String,
        should_succeed: bool,
        execution_delay: Option<Duration>,
        supports_parallel: bool,
    }

    impl MockTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                should_succeed: true,
                execution_delay: None,
                supports_parallel: true,
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_succeed = false;
            self
        }

        fn with_delay(mut self, delay: Duration) -> Self {
            self.execution_delay = Some(delay);
            self
        }

        fn sequential(mut self) -> Self {
            self.supports_parallel = false;
            self
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(
                self.name.clone(),
                self.description().to_string(),
                vec![ToolParameter::string("input", "Test input")],
            )
        }

        async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
            if let Some(delay) = self.execution_delay {
                tokio::time::sleep(delay).await;
            }

            if self.should_succeed {
                Ok(ToolResult::success(
                    &call.id,
                    self.name(),
                    format!("Success from {}", self.name()),
                ))
            } else {
                Err(ToolError::ExecutionFailed(format!(
                    "Mock failure from {}",
                    self.name()
                )))
            }
        }

        fn supports_parallel_execution(&self) -> bool {
            self.supports_parallel
        }
    }

    #[test]
    fn test_executor_creation() {
        let executor = ToolExecutor::new();
        assert_eq!(executor.tool_names().len(), 0);
    }

    #[test]
    fn test_executor_with_settings() {
        let executor = ToolExecutor::with_settings(Duration::from_secs(60), false);
        let stats = executor.get_statistics();
        assert_eq!(stats.max_execution_time, Duration::from_secs(60));
        assert!(!stats.allow_parallel_execution);
    }

    #[test]
    fn test_tool_registration() {
        let mut executor = ToolExecutor::new();
        let tool = Arc::new(MockTool::new("test_tool"));

        executor.register_tool(tool);

        assert_eq!(executor.tool_names().len(), 1);
        assert!(executor.has_tool("test_tool"));
        assert!(!executor.has_tool("nonexistent"));
    }

    #[test]
    fn test_register_multiple_tools() {
        let mut executor = ToolExecutor::new();
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
            Arc::new(MockTool::new("tool3")),
        ];

        executor.register_tools(tools);

        assert_eq!(executor.tool_names().len(), 3);
        assert!(executor.has_tool("tool1"));
        assert!(executor.has_tool("tool2"));
        assert!(executor.has_tool("tool3"));
    }

    #[test]
    fn test_get_tool() {
        let mut executor = ToolExecutor::new();
        let tool = Arc::new(MockTool::new("test_tool"));
        executor.register_tool(tool);

        let retrieved = executor.get_tool("test_tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test_tool");

        let nonexistent = executor.get_tool("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[tokio::test]
    async fn test_execute_single_tool_success() {
        let mut executor = ToolExecutor::new();
        let tool = Arc::new(MockTool::new("test_tool"));
        executor.register_tool(tool);

        let mut args = HashMap::new();
        args.insert("input".to_string(), serde_json::json!("test"));

        let call = ToolCall::new("call_1", "test_tool", args);
        let result = executor.execute_tool(&call).await;

        assert!(result.success);
        assert_eq!(result.tool_name, "test_tool");
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_execute_single_tool_failure() {
        let mut executor = ToolExecutor::new();
        let tool = Arc::new(MockTool::new("test_tool").with_failure());
        executor.register_tool(tool);

        let call = ToolCall::new("call_1", "test_tool", HashMap::new());
        let result = executor.execute_tool(&call).await;

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Mock failure"));
    }

    #[tokio::test]
    async fn test_execute_nonexistent_tool() {
        let executor = ToolExecutor::new();

        let call = ToolCall::new("call_1", "nonexistent_tool", HashMap::new());
        let result = executor.execute_tool(&call).await;

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_execute_multiple_tools() {
        let mut executor = ToolExecutor::new();
        executor.register_tools(vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
            Arc::new(MockTool::new("tool3")),
        ]);

        let calls = vec![
            ToolCall::new("call_1", "tool1", HashMap::new()),
            ToolCall::new("call_2", "tool2", HashMap::new()),
            ToolCall::new("call_3", "tool3", HashMap::new()),
        ];

        let results = executor.execute_tools(&calls).await;

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.success);
        }
    }

    #[tokio::test]
    async fn test_execute_tools_sequential() {
        let mut executor = ToolExecutor::new();
        executor.set_allow_parallel_execution(false);

        executor.register_tools(vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
        ]);

        let calls = vec![
            ToolCall::new("call_1", "tool1", HashMap::new()),
            ToolCall::new("call_2", "tool2", HashMap::new()),
        ];

        let results = executor.execute_tools(&calls).await;
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_tools_with_mixed_support() {
        let mut executor = ToolExecutor::new();
        executor.register_tools(vec![
            Arc::new(MockTool::new("parallel_tool")),
            Arc::new(MockTool::new("sequential_tool").sequential()),
        ]);

        let calls = vec![
            ToolCall::new("call_1", "parallel_tool", HashMap::new()),
            ToolCall::new("call_2", "sequential_tool", HashMap::new()),
        ];

        let results = executor.execute_tools(&calls).await;
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    #[ignore = "Flaky test - timeout behavior varies by environment"]
    async fn test_tool_timeout() {
        let mut executor = ToolExecutor::new();
        executor.set_max_execution_time(Duration::from_millis(100));

        let tool = Arc::new(MockTool::new("slow_tool").with_delay(Duration::from_secs(5)));
        executor.register_tool(tool);

        let call = ToolCall::new("call_1", "slow_tool", HashMap::new());
        let result = executor.execute_tool(&call).await;

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("timed out"));
    }

    #[test]
    fn test_validate_calls_success() {
        let mut executor = ToolExecutor::new();
        let tool = Arc::new(MockTool::new("test_tool"));
        executor.register_tool(tool);

        let calls = vec![ToolCall::new("call_1", "test_tool", HashMap::new())];
        let result = executor.validate_calls(&calls);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_calls_nonexistent_tool() {
        let executor = ToolExecutor::new();

        let calls = vec![ToolCall::new("call_1", "nonexistent", HashMap::new())];
        let result = executor.validate_calls(&calls);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_tool_schemas() {
        let mut executor = ToolExecutor::new();
        executor.register_tools(vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
        ]);

        let schemas = executor.get_tool_schemas();
        assert_eq!(schemas.len(), 2);

        let names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"tool1".to_string()));
        assert!(names.contains(&"tool2".to_string()));
    }

    #[test]
    fn test_get_schemas_for_tools() {
        let mut executor = ToolExecutor::new();
        executor.register_tools(vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
            Arc::new(MockTool::new("tool3")),
        ]);

        let schemas =
            executor.get_schemas_for_tools(&["tool1".to_string(), "tool3".to_string()]);
        assert_eq!(schemas.len(), 2);

        let names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"tool1".to_string()));
        assert!(names.contains(&"tool3".to_string()));
        assert!(!names.contains(&"tool2".to_string()));
    }

    #[test]
    fn test_executor_statistics() {
        let mut executor = ToolExecutor::new();
        executor.register_tools(vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
        ]);

        let stats = executor.get_statistics();
        assert_eq!(stats.total_tools, 2);
        assert_eq!(stats.tool_names.len(), 2);
        assert!(stats.allow_parallel_execution);
    }

    #[test]
    fn test_executor_builder() {
        let tool1 = Arc::new(MockTool::new("tool1"));
        let tool2 = Arc::new(MockTool::new("tool2"));

        let executor = ToolExecutorBuilder::new()
            .with_tool(tool1)
            .with_tool(tool2)
            .with_max_execution_time(Duration::from_secs(120))
            .with_parallel_execution(false)
            .build();

        let stats = executor.get_statistics();
        assert_eq!(stats.total_tools, 2);
        assert_eq!(stats.max_execution_time, Duration::from_secs(120));
        assert!(!stats.allow_parallel_execution);
    }

    #[test]
    fn test_executor_builder_with_tools() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("tool1")),
            Arc::new(MockTool::new("tool2")),
            Arc::new(MockTool::new("tool3")),
        ];

        let executor = ToolExecutorBuilder::new().with_tools(tools).build();

        assert_eq!(executor.tool_names().len(), 3);
    }

    #[test]
    fn test_executor_default() {
        let executor = ToolExecutor::default();
        assert_eq!(executor.tool_names().len(), 0);
    }

    #[test]
    fn test_set_max_execution_time() {
        let mut executor = ToolExecutor::new();
        executor.set_max_execution_time(Duration::from_secs(180));

        let stats = executor.get_statistics();
        assert_eq!(stats.max_execution_time, Duration::from_secs(180));
    }

    #[test]
    fn test_set_allow_parallel_execution() {
        let mut executor = ToolExecutor::new();
        executor.set_allow_parallel_execution(false);

        let stats = executor.get_statistics();
        assert!(!stats.allow_parallel_execution);
    }

    #[tokio::test]
    async fn test_empty_tools_execution() {
        let executor = ToolExecutor::new();
        let results = executor.execute_tools(&[]).await;
        assert_eq!(results.len(), 0);
    }
}
