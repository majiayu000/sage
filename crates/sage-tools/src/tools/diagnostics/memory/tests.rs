//! Tests for memory tools

use sage_core::tools::{Tool, ToolCall};
use serde_json::json;

use super::tool::{RememberTool, SessionNotesTool};

#[tokio::test]
async fn test_remember_tool() {
    let tool = RememberTool::new();

    let call = ToolCall {
        id: "test-1".to_string(),
        name: "Remember".to_string(),
        arguments: json!({
            "memory": "User prefers tabs over spaces",
            "memory_type": "preference"
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.unwrap().contains("Memory stored"));
}

#[tokio::test]
async fn test_session_notes_list() {
    let remember_tool = RememberTool::new();
    let notes_tool = SessionNotesTool::new();

    // Add a memory first
    let add_call = ToolCall {
        id: "test-1".to_string(),
        name: "Remember".to_string(),
        arguments: json!({
            "memory": "Test memory for listing",
            "memory_type": "fact"
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };
    remember_tool.execute(&add_call).await.unwrap();

    // List memories
    let list_call = ToolCall {
        id: "test-2".to_string(),
        name: "SessionNotes".to_string(),
        arguments: json!({
            "action": "list"
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };

    let result = notes_tool.execute(&list_call).await.unwrap();
    assert!(result.success);
}

#[tokio::test]
async fn test_session_notes_stats() {
    let tool = SessionNotesTool::new();

    let call = ToolCall {
        id: "test-1".to_string(),
        name: "SessionNotes".to_string(),
        arguments: json!({
            "action": "stats"
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.unwrap().contains("Memory Statistics"));
}
