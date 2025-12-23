//! Tool execution tests

#![cfg(test)]

use crate::tools::base::concurrency::ConcurrencyMode;
use crate::tools::base::error::ToolError;
use super::test_mocks::MockTool;
use crate::tools::base::tool_trait::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;

#[tokio::test]
async fn test_execute_with_timing_success() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir);

    let call = ToolCall {
        id: "test-1".to_string(),
        name: "mock_tool".to_string(),
        arguments: std::collections::HashMap::new(),
        call_id: None,
    };

    let result = tool.execute_with_timing(&call).await;
    assert!(result.success);
    assert!(result.execution_time_ms.is_some());
    assert!(result.execution_time_ms.unwrap() >= 0);
}

#[tokio::test]
async fn test_execute_with_timing_validation_error() {
    struct ValidatingTool;

    #[async_trait]
    impl Tool for ValidatingTool {
        fn name(&self) -> &str {
            "validating_tool"
        }

        fn description(&self) -> &str {
            "A tool that validates"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("test-id", self.name(), "success"))
        }

        fn validate(&self, _call: &ToolCall) -> Result<(), ToolError> {
            Err(ToolError::ValidationFailed("Validation failed".to_string()))
        }
    }

    let tool = ValidatingTool;
    let call = ToolCall {
        id: "test-2".to_string(),
        name: "validating_tool".to_string(),
        arguments: std::collections::HashMap::new(),
        call_id: None,
    };

    let result = tool.execute_with_timing(&call).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Validation failed"));
    assert!(result.execution_time_ms.is_some());
}

#[test]
fn test_supports_parallel_execution() {
    struct ParallelTool;

    #[async_trait]
    impl Tool for ParallelTool {
        fn name(&self) -> &str {
            "parallel_tool"
        }

        fn description(&self) -> &str {
            "A parallel tool"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("test-id", self.name(), "success"))
        }

        fn concurrency_mode(&self) -> ConcurrencyMode {
            ConcurrencyMode::Parallel
        }
    }

    struct SequentialTool;

    #[async_trait]
    impl Tool for SequentialTool {
        fn name(&self) -> &str {
            "sequential_tool"
        }

        fn description(&self) -> &str {
            "A sequential tool"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("test-id", self.name(), "success"))
        }

        fn concurrency_mode(&self) -> ConcurrencyMode {
            ConcurrencyMode::Sequential
        }
    }

    let parallel = ParallelTool;
    assert!(parallel.supports_parallel_execution());

    let sequential = SequentialTool;
    assert!(!sequential.supports_parallel_execution());
}

#[test]
fn test_max_execution_duration() {
    use std::time::Duration;

    struct CustomTimeTool;

    #[async_trait]
    impl Tool for CustomTimeTool {
        fn name(&self) -> &str {
            "custom_time_tool"
        }

        fn description(&self) -> &str {
            "A tool with custom timeout"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success("test-id", self.name(), "success"))
        }

        fn max_execution_duration(&self) -> Option<Duration> {
            Some(Duration::from_secs(120))
        }
    }

    let tool = CustomTimeTool;
    assert_eq!(
        tool.max_execution_duration(),
        Some(Duration::from_secs(120))
    );
    assert_eq!(tool.max_execution_time(), Some(120));
}

#[test]
fn test_is_read_only_default() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir);
    assert!(!tool.is_read_only());
}

#[test]
fn test_render_call_and_result() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir);

    let mut args = std::collections::HashMap::new();
    args.insert(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    );

    let call = ToolCall {
        id: "test-1".to_string(),
        name: "mock_tool".to_string(),
        arguments: args,
        call_id: None,
    };

    let rendered = tool.render_call(&call);
    assert!(rendered.contains("mock_tool"));
    assert!(rendered.contains("key"));

    let success_result = ToolResult::success("test-id", "mock_tool", "Success!");
    let rendered = tool.render_result(&success_result);
    assert_eq!(rendered, "Success!");

    let error_result = ToolResult::error("test-id", "mock_tool", "Failed!");
    let rendered = tool.render_result(&error_result);
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("Failed!"));
}

#[test]
fn test_requires_user_interaction_default() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir);
    assert!(!tool.requires_user_interaction());
}
