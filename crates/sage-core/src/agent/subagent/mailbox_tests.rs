use std::sync::Arc;

use super::graph::{AgentPath, ChildAgentSpawnRecord, SubAgentGraph, SubAgentGraphError};
use crate::thread_store::{SqliteThreadStore, ThreadRecord, ThreadStatus, ThreadStore};

fn mailbox_graph_with_store()
-> Result<(Arc<SqliteThreadStore>, SubAgentGraph), Box<dyn std::error::Error>> {
    let store = Arc::new(SqliteThreadStore::in_memory()?);
    let graph_store: Arc<dyn ThreadStore> = store.clone();
    Ok((store, SubAgentGraph::new(graph_store)))
}

#[tokio::test]
async fn subagent_mailbox_follow_up_appends_child_message() -> Result<(), Box<dyn std::error::Error>>
{
    let (store, graph) = mailbox_graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-thread",
            "spawn-item",
        ))
        .await?;

    let receipt = graph
        .send_follow_up(&AgentPath::for_child_thread("child-thread"), "next step")
        .await?;

    assert_eq!(receipt.child_thread_id, "child-thread");
    let child = store.read_thread("child-thread").await?;
    let item = child
        .items
        .iter()
        .find(|item| item.item_id == receipt.item_id)
        .expect("follow-up item");
    assert_eq!(item.item_type, "message");
    assert_eq!(item.role.as_deref(), Some("user"));
    assert_eq!(item.status.as_deref(), Some("queued"));
    assert_eq!(item.search_text.as_deref(), Some("next step"));
    assert_eq!(
        item.payload_json
            .as_ref()
            .and_then(|payload| payload.get("kind"))
            .and_then(serde_json::Value::as_str),
        Some("agent.follow_up")
    );
    let follow_ups = graph
        .read_follow_ups_after(&AgentPath::for_child_thread("child-thread"), None)
        .await?;
    assert_eq!(follow_ups.len(), 1);
    assert_eq!(follow_ups[0].message, "next step");
    assert_eq!(follow_ups[0].item_id, receipt.item_id);
    assert_eq!(follow_ups[0].sequence, receipt.sequence);
    Ok(())
}

#[tokio::test]
async fn subagent_mailbox_interrupt_persists_status() -> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = mailbox_graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-thread",
            "spawn-item",
        ))
        .await?;

    graph
        .interrupt_child(
            &AgentPath::for_child_thread("child-thread"),
            Some("parent cancelled"),
        )
        .await?;

    let child = store.read_thread("child-thread").await?;
    assert_eq!(child.thread.status, ThreadStatus::Interrupted);
    let terminal = graph
        .read_terminal_state(&AgentPath::for_child_thread("child-thread"))
        .await?
        .expect("terminal state");
    assert_eq!(terminal.status, ThreadStatus::Interrupted);
    assert_eq!(terminal.reason.as_deref(), Some("parent cancelled"));
    Ok(())
}

#[tokio::test]
async fn subagent_mailbox_rejects_terminal_follow_up() -> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = mailbox_graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    let mut spawn = ChildAgentSpawnRecord::new("parent-thread", "child-thread", "spawn-item");
    spawn.status = ThreadStatus::Completed;
    graph.record_child(spawn).await?;

    let err = graph
        .send_follow_up(&AgentPath::for_child_thread("child-thread"), "too late")
        .await
        .expect_err("terminal child should reject follow-up");

    assert!(matches!(err, SubAgentGraphError::InvalidAgentState { .. }));
    Ok(())
}

#[tokio::test]
async fn subagent_mailbox_terminal_state_survives_reopen() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("threads.sqlite");
    {
        let store = Arc::new(SqliteThreadStore::open(&db_path)?);
        let graph_store: Arc<dyn ThreadStore> = store.clone();
        let graph = SubAgentGraph::new(graph_store);
        store
            .create_thread(ThreadRecord::new("parent-thread"))
            .await?;
        graph
            .record_child(ChildAgentSpawnRecord::new(
                "parent-thread",
                "child-thread",
                "spawn-item",
            ))
            .await?;
        graph
            .record_terminal_state(
                &AgentPath::for_child_thread("child-thread"),
                ThreadStatus::Completed,
                Some("done"),
            )
            .await?;
    }

    let reopened = Arc::new(SqliteThreadStore::open(&db_path)?);
    let graph_store: Arc<dyn ThreadStore> = reopened;
    let graph = SubAgentGraph::new(graph_store);
    let terminal = graph
        .read_terminal_state(&AgentPath::for_child_thread("child-thread"))
        .await?
        .expect("terminal state");
    assert_eq!(terminal.status, ThreadStatus::Completed);
    assert_eq!(terminal.result.as_deref(), Some("done"));
    Ok(())
}

#[tokio::test]
async fn subagent_mailbox_rejects_terminal_write_after_interrupt()
-> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = mailbox_graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-thread",
            "child-thread",
            "spawn-item",
        ))
        .await?;
    let agent_path = AgentPath::for_child_thread("child-thread");
    graph.interrupt_child(&agent_path, Some("stop")).await?;

    let err = graph
        .record_terminal_state(&agent_path, ThreadStatus::Completed, Some("late result"))
        .await
        .expect_err("interrupted child must not be overwritten");

    assert!(matches!(err, SubAgentGraphError::InvalidAgentState { .. }));
    let terminal = graph
        .read_terminal_state(&agent_path)
        .await?
        .expect("interrupt terminal state");
    assert_eq!(terminal.status, ThreadStatus::Interrupted);
    assert_eq!(terminal.reason.as_deref(), Some("stop"));
    Ok(())
}
