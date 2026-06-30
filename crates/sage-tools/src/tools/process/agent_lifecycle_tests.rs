use std::collections::HashMap;
use std::sync::Arc;

use sage_core::agent::subagent::{ChildAgentSpawnRecord, SubAgentGraph};
use sage_core::thread_store::{SqliteThreadStore, ThreadRecord, ThreadStatus, ThreadStore};
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::ToolCall;
use serde_json::json;

use super::{
    super::task::{TaskRegistry, TaskRequest, TaskStatus},
    AgentLifecycleTool,
};

fn create_tool_call(id: &str, args: serde_json::Value) -> ToolCall {
    let arguments = if let serde_json::Value::Object(map) = args {
        map.into_iter().collect()
    } else {
        HashMap::new()
    };

    ToolCall {
        id: id.to_string(),
        name: "AgentLifecycle".to_string(),
        arguments,
        call_id: None,
    }
}

async fn graph_with_parent()
-> Result<(Arc<SqliteThreadStore>, Arc<SubAgentGraph>), Box<dyn std::error::Error>> {
    let store = Arc::new(SqliteThreadStore::in_memory()?);
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    let graph_store: Arc<dyn ThreadStore> = store.clone();
    Ok((store, Arc::new(SubAgentGraph::new(graph_store))))
}

#[test]
fn agent_lifecycle_schema_declares_operations() {
    let tool = AgentLifecycleTool::new();
    let schema = tool.schema();
    assert_eq!(schema.name, "AgentLifecycle");
    let operation = &schema.parameters["properties"]["operation"];
    assert_eq!(operation["enum"], json!(["list", "wait"]));
}

#[tokio::test]
async fn agent_lifecycle_requires_graph() {
    let tool = AgentLifecycleTool::new();
    let err = tool
        .execute(&create_tool_call(
            "list",
            json!({
                "operation": "list",
                "parent_thread_id": "parent-thread"
            }),
        ))
        .await
        .expect_err("graph is required");
    assert!(matches!(err, ToolError::InvalidArguments(_)));
    assert!(err.to_string().contains("SubAgentGraph"));
}

#[tokio::test]
async fn agent_lifecycle_lists_direct_children_and_descendants()
-> Result<(), Box<dyn std::error::Error>> {
    let (_store, graph) = graph_with_parent().await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-a",
            "spawn-a",
        ))
        .await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-b",
            "spawn-b",
        ))
        .await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "child-a",
            "grandchild-a",
            "spawn-grandchild",
        ))
        .await?;
    let tool = AgentLifecycleTool::with_graph(graph);

    let direct = tool
        .execute(&create_tool_call(
            "list-direct",
            json!({
                "operation": "list",
                "parent_thread_id": "parent-thread"
            }),
        ))
        .await?;
    assert!(direct.success);
    assert_eq!(direct.metadata.get("operation"), Some(&json!("list")));
    let direct_children = direct
        .metadata
        .get("children")
        .and_then(|value| value.as_array())
        .expect("children metadata");
    assert_eq!(direct_children.len(), 2);
    assert_eq!(direct_children[0]["child_thread_id"], json!("child-a"));
    assert_eq!(direct_children[1]["child_thread_id"], json!("child-b"));

    let descendants = tool
        .execute(&create_tool_call(
            "list-descendants",
            json!({
                "operation": "list",
                "parent_thread_id": "parent-thread",
                "depth": "descendants"
            }),
        ))
        .await?;
    let descendant_children = descendants
        .metadata
        .get("children")
        .and_then(|value| value.as_array())
        .expect("children metadata");
    assert_eq!(descendant_children.len(), 3);
    assert_eq!(
        descendant_children
            .iter()
            .map(|child| child["child_thread_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["child-a", "child-b", "grandchild-a"]
    );
    Ok(())
}

#[tokio::test]
async fn agent_lifecycle_wait_returns_terminal_status() -> Result<(), Box<dyn std::error::Error>> {
    let (_store, graph) = graph_with_parent().await?;
    let mut spawn = ChildAgentSpawnRecord::new("parent-thread", "child-done", "spawn-done");
    spawn.status = ThreadStatus::Completed;
    graph.record_child(spawn).await?;
    let tool = AgentLifecycleTool::with_graph(graph);

    let result = tool
        .execute(&create_tool_call(
            "wait-done",
            json!({
                "operation": "wait",
                "agent_path": "agent://child-done",
                "timeout": 1
            }),
        ))
        .await?;
    assert!(result.success);
    assert_eq!(result.metadata.get("operation"), Some(&json!("wait")));
    assert_eq!(result.metadata.get("status"), Some(&json!("completed")));
    assert_eq!(
        result
            .metadata
            .get("agent")
            .and_then(|agent| agent.get("agent_path")),
        Some(&json!("agent://child-done"))
    );
    Ok(())
}

#[tokio::test]
async fn agent_lifecycle_wait_uses_shared_registry_terminal_status()
-> Result<(), Box<dyn std::error::Error>> {
    let (_store, graph) = graph_with_parent().await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-registry-done",
            "spawn-registry-done",
        ))
        .await?;
    let registry = Arc::new(TaskRegistry::new());
    registry.add_task(TaskRequest {
        id: "child-registry-done".to_string(),
        description: "Done child".to_string(),
        prompt: "Return".to_string(),
        subagent_type: "Plan".to_string(),
        model: None,
        run_in_background: true,
        resume: None,
        status: TaskStatus::Completed,
        result: Some("registry final result".to_string()),
    });
    let tool = AgentLifecycleTool::with_task_registry_and_graph(registry, graph);

    let result = tool
        .execute(&create_tool_call(
            "wait-registry-done",
            json!({
                "operation": "wait",
                "agent_path": "agent://child-registry-done",
                "timeout": 1
            }),
        ))
        .await?;
    assert!(result.success);
    assert_eq!(result.metadata.get("status"), Some(&json!("completed")));
    assert_eq!(result.metadata.get("graph_status"), Some(&json!("active")));
    assert_eq!(
        result.metadata.get("task_status"),
        Some(&json!("completed"))
    );
    assert_eq!(
        result
            .metadata
            .get("agent")
            .and_then(|agent| agent.get("final_result")),
        Some(&json!("registry final result"))
    );
    Ok(())
}

#[tokio::test]
async fn agent_lifecycle_wait_times_out_for_active_child() -> Result<(), Box<dyn std::error::Error>>
{
    let (_store, graph) = graph_with_parent().await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-active",
            "spawn-active",
        ))
        .await?;
    let tool = AgentLifecycleTool::with_graph(graph);

    let result = tool
        .execute(&create_tool_call(
            "wait-active",
            json!({
                "operation": "wait",
                "agent_path": "agent://child-active",
                "timeout": 1
            }),
        ))
        .await?;
    assert!(!result.success);
    assert_eq!(result.metadata.get("error_code"), Some(&json!("timeout")));
    assert_eq!(
        result.metadata.get("agent_path"),
        Some(&json!("agent://child-active"))
    );
    assert_eq!(result.metadata.get("last_status"), Some(&json!("active")));
    Ok(())
}

#[test]
fn agent_lifecycle_does_not_inherit_into_subagent_runner() {
    assert!(!AgentLifecycleTool::new().include_in_subagent_runner());
}
