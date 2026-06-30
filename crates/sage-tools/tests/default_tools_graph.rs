use sage_core::agent::subagent::{AgentPath, ChildAgentSpawnRecord, SubAgentGraph};
use sage_core::skills::SkillRegistry;
use sage_core::thread_store::{SqliteThreadStore, ThreadRecord, ThreadStore};
use sage_core::tools::base::Tool;
use sage_core::tools::permission::ToolContext;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::AgentLifecycleTool;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

fn graph_default_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
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
async fn default_tools_share_graph_backed_task_registry() -> Result<(), Box<dyn std::error::Error>>
{
    let workspace = tempfile::tempdir()?;
    let store = Arc::new(SqliteThreadStore::in_memory()?);
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    let mut skill_registry = SkillRegistry::new(workspace.path());
    skill_registry.register_builtins();
    let skill_registry = Arc::new(RwLock::new(skill_registry));

    let thread_store: Arc<dyn ThreadStore> = store.clone();
    let tools = sage_tools::get_default_tools_with_context_and_thread_store(
        workspace.path(),
        skill_registry,
        thread_store.clone(),
    );
    assert!(tools.iter().any(|tool| tool.name() == "TaskOutput"));
    assert!(tools.iter().any(|tool| tool.name() == "AgentLifecycle"));
    let context = ToolContext::new(workspace.path().to_path_buf()).with_session_id("parent-thread");

    let spawn = {
        let task = tools
            .iter()
            .find(|tool| tool.name() == "Task")
            .ok_or("Task tool should be registered")?;
        assert!(task.include_in_subagent_runner());
        task.execute_with_context(
            &graph_default_tool_call(
                "spawn-item",
                "Task",
                json!({
                    "description": "Plan implementation",
                    "prompt": "Design test plan",
                    "subagent_type": "Plan",
                    "run_in_background": true,
                    "resume": "task_graph_default"
                }),
            ),
            &context,
        )
        .await?
    };
    assert!(spawn.success);
    let agent_path = spawn
        .metadata
        .get("agent_path")
        .and_then(|value| value.as_str())
        .ok_or("expected graph-backed agent_path metadata")?
        .to_string();
    assert_eq!(agent_path, "agent://task_graph_default");
    drop(tools);

    let graph_store: Arc<dyn ThreadStore> = store.clone();
    let graph = SubAgentGraph::new(graph_store);
    let summary = graph
        .read_child(&AgentPath::from_raw_path(agent_path.as_str())?)
        .await?;
    assert_eq!(summary.parent_thread_id, "parent-thread");
    assert_eq!(summary.spawn_item_id, "spawn-item");

    let tools_after_drop = sage_tools::get_default_tools_with_context_and_thread_store(
        workspace.path(),
        Arc::new(RwLock::new(SkillRegistry::new(workspace.path()))),
        thread_store,
    );
    let task_output = tools_after_drop
        .iter()
        .find(|tool| tool.name() == "TaskOutput")
        .ok_or("TaskOutput tool should be registered")?;
    assert!(!task_output.include_in_subagent_runner());
    let agent_lifecycle = tools_after_drop
        .iter()
        .find(|tool| tool.name() == "AgentLifecycle")
        .ok_or("AgentLifecycle tool should be registered")?;
    assert!(agent_lifecycle.is_read_only());

    let output = task_output
        .execute(&graph_default_tool_call(
            "task-output",
            "TaskOutput",
            json!({ "agent_path": agent_path }),
        ))
        .await?;
    assert!(output.success);
    assert_eq!(
        output.metadata.get("task_id"),
        Some(&json!("task_graph_default"))
    );
    assert_eq!(output.metadata.get("agent_path"), Some(&json!(agent_path)));

    let listed = agent_lifecycle
        .execute(&graph_default_tool_call(
            "agent-list",
            "AgentLifecycle",
            json!({
                "operation": "list",
                "parent_thread_id": "parent-thread"
            }),
        ))
        .await?;
    let children = listed
        .metadata
        .get("children")
        .and_then(|value| value.as_array())
        .ok_or("children metadata should be present")?;
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["agent_path"], json!(agent_path));
    Ok(())
}

#[tokio::test]
async fn graph_default_toolsets_do_not_share_task_registries()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir()?;
    let store_a = Arc::new(SqliteThreadStore::in_memory()?);
    let store_b = Arc::new(SqliteThreadStore::in_memory()?);
    store_a
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    store_b
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;

    let mut skill_registry_a = SkillRegistry::new(workspace.path());
    skill_registry_a.register_builtins();
    let mut skill_registry_b = SkillRegistry::new(workspace.path());
    skill_registry_b.register_builtins();

    let thread_store_a: Arc<dyn ThreadStore> = store_a.clone();
    let thread_store_b: Arc<dyn ThreadStore> = store_b.clone();
    let tools_a = sage_tools::get_default_tools_with_context_and_thread_store(
        workspace.path(),
        Arc::new(RwLock::new(skill_registry_a)),
        thread_store_a,
    );
    let tools_b = sage_tools::get_default_tools_with_context_and_thread_store(
        workspace.path(),
        Arc::new(RwLock::new(skill_registry_b)),
        thread_store_b,
    );
    let task_a = tools_a
        .iter()
        .find(|tool| tool.name() == "Task")
        .ok_or("Task tool should be registered")?;
    let task_output_b = tools_b
        .iter()
        .find(|tool| tool.name() == "TaskOutput")
        .ok_or("TaskOutput tool should be registered")?;
    let context = ToolContext::new(workspace.path().to_path_buf()).with_session_id("parent-thread");

    let spawn = task_a
        .execute_with_context(
            &graph_default_tool_call(
                "spawn-a",
                "Task",
                json!({
                    "description": "Plan implementation",
                    "prompt": "Design test plan",
                    "subagent_type": "Plan",
                    "run_in_background": true,
                    "resume": "shared_task_id"
                }),
            ),
            &context,
        )
        .await?;
    assert!(spawn.success);

    let graph_store_b: Arc<dyn ThreadStore> = store_b.clone();
    let graph_b = SubAgentGraph::new(graph_store_b);
    graph_b
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "shared_task_id",
            "spawn-b",
        ))
        .await?;

    let err = task_output_b
        .execute(&graph_default_tool_call(
            "task-output-b",
            "TaskOutput",
            json!({ "agent_path": "agent://shared_task_id" }),
        ))
        .await
        .expect_err("second configured toolset must not read first toolset task registry");
    assert!(err.to_string().contains("shared_task_id"));
    Ok(())
}

#[tokio::test]
async fn agent_lifecycle_wait_surfaces_non_child_graph_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(SqliteThreadStore::in_memory()?);
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    store
        .create_thread(ThreadRecord::new("orphan-child"))
        .await?;
    let graph_store: Arc<dyn ThreadStore> = store;
    let tool = AgentLifecycleTool::with_graph(Arc::new(SubAgentGraph::new(graph_store)));

    let err = tool
        .execute(&graph_default_tool_call(
            "wait-orphan",
            "AgentLifecycle",
            json!({
                "operation": "wait",
                "agent_path": "agent://orphan-child",
                "timeout": 1
            }),
        ))
        .await
        .expect_err("non-child graph errors should not be reported as missing agents");

    assert!(
        err.to_string()
            .contains("failed to read agent agent://orphan-child")
    );
    assert!(err.to_string().contains("invalid agent path"));
    Ok(())
}
