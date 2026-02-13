use super::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[tokio::test]
async fn test_kill_shell_not_found() {
    let registry = Arc::new(Mutex::new(HashMap::new()));
    let tool = KillShellTool::with_registry(registry);

    let call = create_tool_call(
        "test-1",
        "KillShell",
        json!({
            "shell_id": "nonexistent_shell"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    match result {
        Err(ToolError::NotFound(msg)) => {
            assert!(msg.contains("nonexistent_shell"));
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_kill_shell_missing_parameter() {
    let tool = KillShellTool::new();
    let call = create_tool_call("test-2", "KillShell", json!({}));

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
async fn test_kill_shell_empty_id() {
    let tool = KillShellTool::new();
    let call = create_tool_call(
        "test-3",
        "KillShell",
        json!({
            "shell_id": ""
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());

    match result {
        Err(ToolError::InvalidArguments(msg)) => {
            assert!(msg.contains("empty"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

#[tokio::test]
async fn test_kill_shell_invalid_id_format() {
    let tool = KillShellTool::new();
    let call = create_tool_call(
        "test-4",
        "KillShell",
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
async fn test_kill_shell_validation_success() {
    let tool = KillShellTool::new();

    // Valid shell IDs
    let valid_ids = vec!["shell_1", "background-shell-2", "shell123", "SHELL_ABC"];

    for id in valid_ids {
        let call = create_tool_call(
            "test-5",
            "KillShell",
            json!({
                "shell_id": id
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_ok(), "Failed to validate ID: {}", id);
    }
}

#[test]
fn test_kill_shell_schema() {
    let tool = KillShellTool::new();
    let schema = tool.schema();

    assert_eq!(schema.name, "KillShell");
    assert!(!schema.description.is_empty());

    // Check that shell_id parameter exists
    let params = schema.parameters;
    assert!(params.get("properties").is_some());
    assert!(params.get("required").is_some());
}

#[test]
fn test_kill_shell_tool_properties() {
    let tool = KillShellTool::new();

    assert_eq!(tool.name(), "KillShell");
    assert!(!tool.description().is_empty());
    assert_eq!(
        tool.max_execution_duration(),
        Some(std::time::Duration::from_secs(30))
    );
    assert!(tool.supports_parallel_execution());
    assert!(!tool.is_read_only());
}

#[cfg(unix)]
#[tokio::test]
async fn test_kill_shell_with_mock_process() {
    use std::process::Command;

    // Start a simple sleep process
    let mut child = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("Failed to spawn sleep process");

    let pid = child.id();

    // Register the shell
    let registry = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut shells = registry.lock().await;
        shells.insert("test_shell".to_string(), pid);
    }

    let tool = KillShellTool::with_registry(registry.clone());

    let call = create_tool_call(
        "test-6",
        "KillShell",
        json!({
            "shell_id": "test_shell"
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_ok(), "Failed to execute: {:?}", result);

    let tool_result = result.unwrap();
    assert!(tool_result.success);
    assert!(
        tool_result
            .output
            .unwrap()
            .contains("Successfully terminated")
    );

    // Verify the shell was removed from registry
    let shells = registry.lock().await;
    assert!(!shells.contains_key("test_shell"));

    // Clean up: ensure child is killed
    let _ = child.kill();
    let _ = child.wait();
}
