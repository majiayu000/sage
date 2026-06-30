use sage_core::agent::subagent::{ChildAgentSpawnRecord, SubAgentGraph};
use sage_core::thread_store::{SqliteThreadStore, ThreadRecord, ThreadStatus, ThreadStore};
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::AgentMessagingTool;
use crate::tools::process::task::{TaskRegistry, TaskRequest, TaskStatus};

fn messaging_tool_call(id: &str, args: serde_json::Value) -> ToolCall {
    let arguments = if let serde_json::Value::Object(map) = args {
        map.into_iter().collect()
    } else {
        HashMap::new()
    };
    ToolCall {
        id: id.to_string(),
        name: "AgentMessaging".to_string(),
        arguments,
        call_id: None,
    }
}

async fn messaging_tool_fixture() -> Result<
    (
        Arc<SqliteThreadStore>,
        Arc<TaskRegistry>,
        AgentMessagingTool,
    ),
    Box<dyn std::error::Error>,
> {
    let store = Arc::new(SqliteThreadStore::in_memory()?);
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    let graph_store: Arc<dyn ThreadStore> = store.clone();
    let graph = Arc::new(SubAgentGraph::new(graph_store));
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-thread",
            "spawn-item",
        ))
        .await?;
    let registry = Arc::new(TaskRegistry::new());
    let tool = AgentMessagingTool::with_task_registry_and_graph(registry.clone(), graph);
    Ok((store, registry, tool))
}

#[tokio::test]
async fn agent_messaging_follow_up_returns_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let (store, registry, tool) = messaging_tool_fixture().await?;
    registry.add_task(TaskRequest {
        id: "child-thread".to_string(),
        description: "Child work".to_string(),
        prompt: "Do work".to_string(),
        subagent_type: "Plan".to_string(),
        model: None,
        run_in_background: true,
        resume: None,
        status: TaskStatus::Running,
        result: None,
    });

    let result = tool
        .execute(&messaging_tool_call(
            "follow",
            json!({
                "operation": "follow_up",
                "agent_path": "agent://child-thread",
                "message": "continue with tests"
            }),
        ))
        .await?;

    assert!(result.success);
    assert_eq!(result.metadata.get("operation"), Some(&json!("follow_up")));
    assert_eq!(result.metadata.get("status"), Some(&json!("queued")));
    assert_eq!(
        result.metadata.get("delivery"),
        Some(&json!("live_mailbox"))
    );
    let child = store.read_thread("child-thread").await?;
    assert!(child.items.iter().any(|item| {
        item.payload_json
            .as_ref()
            .and_then(|payload| payload.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("agent.follow_up")
    }));
    Ok(())
}

#[tokio::test]
async fn agent_messaging_follow_up_without_live_task_returns_unsupported()
-> Result<(), Box<dyn std::error::Error>> {
    let (store, _registry, tool) = messaging_tool_fixture().await?;

    let result = tool
        .execute(&messaging_tool_call(
            "follow-no-live-task",
            json!({
                "operation": "follow_up",
                "agent_path": "agent://child-thread",
                "message": "continue with tests"
            }),
        ))
        .await?;

    assert!(!result.success);
    assert_eq!(
        result.metadata.get("error_code"),
        Some(&json!("unsupported_follow_up"))
    );
    let child = store.read_thread("child-thread").await?;
    assert!(!child.items.iter().any(|item| {
        item.payload_json
            .as_ref()
            .and_then(|payload| payload.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("agent.follow_up")
    }));
    Ok(())
}

#[tokio::test]
async fn agent_messaging_interrupt_updates_graph_and_registry()
-> Result<(), Box<dyn std::error::Error>> {
    let (store, registry, tool) = messaging_tool_fixture().await?;
    registry.add_task(TaskRequest {
        id: "child-thread".to_string(),
        description: "Child work".to_string(),
        prompt: "Do work".to_string(),
        subagent_type: "Plan".to_string(),
        model: None,
        run_in_background: true,
        resume: None,
        status: TaskStatus::Running,
        result: None,
    });

    let result = tool
        .execute(&messaging_tool_call(
            "interrupt",
            json!({
                "operation": "interrupt",
                "agent_path": "agent://child-thread",
                "reason": "stop now"
            }),
        ))
        .await?;

    assert!(result.success);
    assert_eq!(result.metadata.get("status"), Some(&json!("interrupted")));
    assert_eq!(
        result.metadata.get("interrupted_live_task"),
        Some(&json!(true))
    );
    let task = registry.get_task("child-thread").expect("task");
    assert_eq!(task.status, TaskStatus::Interrupted);
    assert_eq!(task.result.as_deref(), Some("stop now"));
    let child = store.read_thread("child-thread").await?;
    assert_eq!(child.thread.status, ThreadStatus::Interrupted);
    Ok(())
}

#[tokio::test]
async fn agent_messaging_interrupt_without_reason_uses_default_error()
-> Result<(), Box<dyn std::error::Error>> {
    let (_store, registry, tool) = messaging_tool_fixture().await?;
    registry.add_task(TaskRequest {
        id: "child-thread".to_string(),
        description: "Child work".to_string(),
        prompt: "Do work".to_string(),
        subagent_type: "Plan".to_string(),
        model: None,
        run_in_background: true,
        resume: None,
        status: TaskStatus::Running,
        result: None,
    });

    let result = tool
        .execute(&messaging_tool_call(
            "interrupt-default",
            json!({
                "operation": "interrupt",
                "agent_path": "agent://child-thread"
            }),
        ))
        .await?;

    assert!(result.success);
    let task = registry.get_task("child-thread").expect("task");
    assert_eq!(task.status, TaskStatus::Interrupted);
    assert_eq!(task.result.as_deref(), Some("interrupted by parent"));
    Ok(())
}

#[tokio::test]
async fn agent_messaging_follow_up_terminal_child_returns_structured_invalid_state()
-> Result<(), Box<dyn std::error::Error>> {
    let (store, _registry, tool) = messaging_tool_fixture().await?;
    ThreadStore::set_thread_status(store.as_ref(), "child-thread", ThreadStatus::Completed).await?;

    let result = tool
        .execute(&messaging_tool_call(
            "follow-terminal",
            json!({
                "operation": "follow_up",
                "agent_path": "agent://child-thread",
                "message": "too late"
            }),
        ))
        .await?;

    assert!(!result.success);
    assert_eq!(
        result.metadata.get("error_code"),
        Some(&json!("invalid_state"))
    );
    assert_eq!(result.metadata.get("status"), Some(&json!("completed")));
    assert_eq!(result.metadata.get("retryable"), Some(&json!(false)));
    Ok(())
}
