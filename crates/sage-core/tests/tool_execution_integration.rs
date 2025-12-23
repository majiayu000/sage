//! Integration tests for tool execution flow

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::executor::{ToolExecutor, ToolExecutorBuilder};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// Mock echo tool
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes back the input message"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string("message", "The message to echo")],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let message = call.get_string("message").unwrap_or_default();
        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!("Echo: {}", message),
        ))
    }
}

// Mock calculator tool
struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Performs basic arithmetic operations"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::number("a", "First number"),
                ToolParameter::number("b", "Second number"),
                ToolParameter::string("operation", "Operation: add, subtract, multiply, divide"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let a = call.get_number("a").unwrap_or(0.0);
        let b = call.get_number("b").unwrap_or(0.0);
        let operation = call.get_string("operation").unwrap_or_default();

        let result = match operation.as_str() {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(ToolError::ExecutionFailed("Division by zero".to_string()));
                }
                a / b
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown operation: {}",
                    operation
                )));
            }
        };

        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!("{} {} {} = {}", a, operation, b, result),
        ))
    }
}

// Mock file reader tool (read-only)
struct FileReaderTool;

#[async_trait]
impl Tool for FileReaderTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Reads a file from the filesystem"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string("path", "Path to the file to read")],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let path = call.get_string("path").unwrap_or_default();

        // Mock file reading - in real implementation would read from disk
        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!("Contents of {}: [mock file content]", path),
        ))
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_single_tool_execution() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .build();

    let mut args = HashMap::new();
    args.insert("message".to_string(), serde_json::json!("Hello, World!"));

    let call = ToolCall::new("call_1", "echo", args);
    let result = executor.execute_tool(&call).await;

    assert!(result.success);
    assert_eq!(result.tool_name, "echo");
    assert!(result.output.unwrap().contains("Hello, World!"));
}

#[tokio::test]
async fn test_multiple_tools_parallel_execution() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .with_tool(Arc::new(CalculatorTool))
        .with_tool(Arc::new(FileReaderTool))
        .with_parallel_execution(true)
        .build();

    let calls = vec![
        {
            let mut args = HashMap::new();
            args.insert("message".to_string(), serde_json::json!("Test 1"));
            ToolCall::new("call_1", "echo", args)
        },
        {
            let mut args = HashMap::new();
            args.insert("a".to_string(), serde_json::json!(10));
            args.insert("b".to_string(), serde_json::json!(5));
            args.insert("operation".to_string(), serde_json::json!("add"));
            ToolCall::new("call_2", "calculator", args)
        },
        {
            let mut args = HashMap::new();
            args.insert("path".to_string(), serde_json::json!("/test/file.txt"));
            ToolCall::new("call_3", "read_file", args)
        },
    ];

    let results = executor.execute_tools(&calls).await;

    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.success);
    }
}

#[tokio::test]
async fn test_calculator_operations() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(CalculatorTool))
        .build();

    // Test addition
    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(10));
    args.insert("b".to_string(), serde_json::json!(5));
    args.insert("operation".to_string(), serde_json::json!("add"));
    let call = ToolCall::new("call_1", "calculator", args);
    let result = executor.execute_tool(&call).await;
    assert!(result.success);
    assert!(result.output.unwrap().contains("15"));

    // Test subtraction
    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(10));
    args.insert("b".to_string(), serde_json::json!(5));
    args.insert("operation".to_string(), serde_json::json!("subtract"));
    let call = ToolCall::new("call_2", "calculator", args);
    let result = executor.execute_tool(&call).await;
    assert!(result.success);
    assert!(result.output.unwrap().contains("5"));

    // Test multiplication
    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(10));
    args.insert("b".to_string(), serde_json::json!(5));
    args.insert("operation".to_string(), serde_json::json!("multiply"));
    let call = ToolCall::new("call_3", "calculator", args);
    let result = executor.execute_tool(&call).await;
    assert!(result.success);
    assert!(result.output.unwrap().contains("50"));

    // Test division
    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(10));
    args.insert("b".to_string(), serde_json::json!(5));
    args.insert("operation".to_string(), serde_json::json!("divide"));
    let call = ToolCall::new("call_4", "calculator", args);
    let result = executor.execute_tool(&call).await;
    assert!(result.success);
    assert!(result.output.unwrap().contains("2"));
}

#[tokio::test]
async fn test_calculator_division_by_zero() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(CalculatorTool))
        .build();

    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(10));
    args.insert("b".to_string(), serde_json::json!(0));
    args.insert("operation".to_string(), serde_json::json!("divide"));
    let call = ToolCall::new("call_1", "calculator", args);
    let result = executor.execute_tool(&call).await;

    assert!(!result.success);
    assert!(result.error.unwrap().contains("Division by zero"));
}

#[tokio::test]
async fn test_calculator_invalid_operation() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(CalculatorTool))
        .build();

    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(10));
    args.insert("b".to_string(), serde_json::json!(5));
    args.insert("operation".to_string(), serde_json::json!("power"));
    let call = ToolCall::new("call_1", "calculator", args);
    let result = executor.execute_tool(&call).await;

    assert!(!result.success);
    assert!(result.error.unwrap().contains("Unknown operation"));
}

#[tokio::test]
async fn test_tool_schemas() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .with_tool(Arc::new(CalculatorTool))
        .with_tool(Arc::new(FileReaderTool))
        .build();

    let schemas = executor.get_tool_schemas();
    assert_eq!(schemas.len(), 3);

    let names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"echo".to_string()));
    assert!(names.contains(&"calculator".to_string()));
    assert!(names.contains(&"read_file".to_string()));
}

#[tokio::test]
async fn test_sequential_execution() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .with_tool(Arc::new(CalculatorTool))
        .with_parallel_execution(false)
        .build();

    let calls = vec![
        {
            let mut args = HashMap::new();
            args.insert("message".to_string(), serde_json::json!("First"));
            ToolCall::new("call_1", "echo", args)
        },
        {
            let mut args = HashMap::new();
            args.insert("message".to_string(), serde_json::json!("Second"));
            ToolCall::new("call_2", "echo", args)
        },
    ];

    let results = executor.execute_tools(&calls).await;

    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert!(results[1].success);
}

#[tokio::test]
async fn test_execution_timing() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .build();

    let mut args = HashMap::new();
    args.insert("message".to_string(), serde_json::json!("Timed test"));
    let call = ToolCall::new("call_1", "echo", args);
    let result = executor.execute_tool(&call).await;

    assert!(result.success);
    assert!(result.execution_time_ms.is_some());
    assert!(result.execution_time_ms.unwrap() >= 0);
}

#[tokio::test]
async fn test_tool_validation() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .with_tool(Arc::new(CalculatorTool))
        .build();

    // Valid calls
    let valid_calls = vec![
        ToolCall::new("call_1", "echo", HashMap::new()),
        ToolCall::new("call_2", "calculator", HashMap::new()),
    ];
    assert!(executor.validate_calls(&valid_calls).is_ok());

    // Invalid call (nonexistent tool)
    let invalid_calls = vec![ToolCall::new("call_1", "nonexistent_tool", HashMap::new())];
    assert!(executor.validate_calls(&invalid_calls).is_err());
}

#[tokio::test]
async fn test_get_specific_tool_schemas() {
    let executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(EchoTool))
        .with_tool(Arc::new(CalculatorTool))
        .with_tool(Arc::new(FileReaderTool))
        .build();

    let schemas =
        executor.get_schemas_for_tools(&vec!["echo".to_string(), "calculator".to_string()]);

    assert_eq!(schemas.len(), 2);
    let names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"echo".to_string()));
    assert!(names.contains(&"calculator".to_string()));
    assert!(!names.contains(&"read_file".to_string()));
}

#[tokio::test]
async fn test_tool_call_helpers() {
    let mut args = HashMap::new();
    args.insert("string_arg".to_string(), serde_json::json!("test"));
    args.insert("bool_arg".to_string(), serde_json::json!(true));
    args.insert("number_arg".to_string(), serde_json::json!(42.5));

    let call = ToolCall::new("call_1", "test_tool", args);

    assert_eq!(call.get_string("string_arg"), Some("test".to_string()));
    assert_eq!(call.get_bool("bool_arg"), Some(true));
    assert_eq!(call.get_number("number_arg"), Some(42.5));
    assert_eq!(call.get_string("nonexistent"), None);
}
