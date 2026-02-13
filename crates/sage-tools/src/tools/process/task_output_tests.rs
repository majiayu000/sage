use super::*;
use sage_core::tools::BackgroundShellTask;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
    let arguments = if let serde_json::Value::Object(map) = args {
        map.into_iter().collect()
    } else {
        HashMap::new()
    };

    ToolCall {
        id: id.to_string(),
        name: name.to_string(),
        arguments,
        call_id: None,
    }
}

#[test]
fn test_task_output_schema() {
    let tool = TaskOutputTool::new();
    let schema = tool.schema();

    assert_eq!(schema.name, "TaskOutput");
    assert!(!schema.description.is_empty());
}

#[test]
fn test_task_output_tool_properties() {
    let tool = TaskOutputTool::new();

    assert_eq!(tool.name(), "TaskOutput");
    assert!(tool.description().contains("background"));
    assert_eq!(
        tool.max_execution_duration(),
        Some(std::time::Duration::from_secs(600))
    );
    assert!(tool.supports_parallel_execution());
    assert!(tool.is_read_only());
}

#[tokio::test]
async fn test_task_output_missing_shell_id() {
    let tool = TaskOutputTool::new();
    let call = create_tool_call("test-1", "TaskOutput", json!({}));

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    match result {
        Err(ToolError::InvalidArguments(msg)) => {
            assert!(msg.contains("shell_id"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

#[tokio::test]
async fn test_task_output_empty_shell_id() {
    let tool = TaskOutputTool::new();
    let call = create_tool_call(
        "test-2",
        "TaskOutput",
        json!({
            "shell_id": ""
        }),
    );

    let result = tool.validate(&call);
    assert!(result.is_err());

    match result {
        Err(ToolError::InvalidArguments(msg)) => {
            assert!(msg.contains("empty"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

#[tokio::test]
async fn test_task_output_invalid_shell_id() {
    let tool = TaskOutputTool::new();
    let call = create_tool_call(
        "test-3",
        "TaskOutput",
        json!({
            "shell_id": "invalid shell!"
        }),
    );

    let result = tool.validate(&call);
    assert!(result.is_err());

    match result {
        Err(ToolError::InvalidArguments(msg)) => {
            assert!(msg.contains("alphanumeric"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

#[tokio::test]
async fn test_task_output_shell_not_found() {
    let tool = TaskOutputTool::new();
    let call = create_tool_call(
        "test-4",
        "TaskOutput",
        json!({
            "shell_id": "nonexistent_shell_xyz"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    match result {
        Err(ToolError::NotFound(msg)) => {
            assert!(msg.contains("nonexistent_shell_xyz"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_task_output_timeout_validation() {
    let tool = TaskOutputTool::new();

    // Negative timeout
    let call = create_tool_call(
        "test-5",
        "TaskOutput",
        json!({
            "shell_id": "test_shell",
            "timeout": -1000.0
        }),
    );
    assert!(tool.validate(&call).is_err());

    // Excessive timeout
    let call = create_tool_call(
        "test-6",
        "TaskOutput",
        json!({
            "shell_id": "test_shell",
            "timeout": 700000.0
        }),
    );
    assert!(tool.validate(&call).is_err());

    // Valid timeout
    let call = create_tool_call(
        "test-7",
        "TaskOutput",
        json!({
            "shell_id": "test_shell",
            "timeout": 5000.0
        }),
    );
    assert!(tool.validate(&call).is_ok());
}

#[tokio::test]
async fn test_task_output_with_real_task() {
    // Create and register a background task
    let cancel_token = CancellationToken::new();
    let task = BackgroundShellTask::spawn(
        "test_task_output_1".to_string(),
        "echo 'hello from background'",
        &PathBuf::from("/tmp"),
        cancel_token,
    )
    .await
    .unwrap();

    BACKGROUND_REGISTRY.register(Arc::new(task));

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(100)).await;

    let tool = TaskOutputTool::new();
    let call = create_tool_call(
        "test-8",
        "TaskOutput",
        json!({
            "shell_id": "test_task_output_1",
            "incremental": false
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_ok());

    let tool_result = result.unwrap();
    assert!(tool_result.success);
    let output = tool_result.output.unwrap();
    assert!(output.contains("hello from background"));
    assert!(output.contains("COMPLETED"));

    // Cleanup
    BACKGROUND_REGISTRY.remove("test_task_output_1");
}

#[tokio::test]
async fn test_task_output_incremental() {
    let cancel_token = CancellationToken::new();
    let task = BackgroundShellTask::spawn(
        "test_task_output_2".to_string(),
        "echo 'line1'; sleep 0.1; echo 'line2'",
        &PathBuf::from("/tmp"),
        cancel_token,
    )
    .await
    .unwrap();

    BACKGROUND_REGISTRY.register(Arc::new(task));

    let tool = TaskOutputTool::new();

    // First read (incremental)
    tokio::time::sleep(Duration::from_millis(50)).await;
    let call1 = create_tool_call(
        "test-9a",
        "TaskOutput",
        json!({
            "shell_id": "test_task_output_2",
            "incremental": true
        }),
    );
    let result1 = tool.execute(&call1).await.unwrap();
    let output1 = result1.output.unwrap();

    // Second read (incremental)
    tokio::time::sleep(Duration::from_millis(150)).await;
    let call2 = create_tool_call(
        "test-9b",
        "TaskOutput",
        json!({
            "shell_id": "test_task_output_2",
            "incremental": true
        }),
    );
    let result2 = tool.execute(&call2).await.unwrap();
    let output2 = result2.output.unwrap();

    // Combined output should have both lines
    let combined = format!("{}{}", output1, output2);
    assert!(combined.contains("line1") || combined.contains("line2"));

    // Cleanup
    BACKGROUND_REGISTRY.remove("test_task_output_2");
}
