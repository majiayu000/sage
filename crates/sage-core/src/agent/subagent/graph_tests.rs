use std::sync::Arc;

use super::graph::{
    AgentGraphListQuery, AgentPath, ChildAgentSpawnRecord, SubAgentGraph, SubAgentGraphError,
};
use crate::thread_store::{
    SqliteThreadStore, ThreadLineage, ThreadRecord, ThreadStatus, ThreadStore, ThreadStoreError,
};
use tempfile::TempDir;

fn graph_with_store() -> Result<(Arc<SqliteThreadStore>, SubAgentGraph), Box<dyn std::error::Error>>
{
    let store = Arc::new(SqliteThreadStore::in_memory()?);
    let graph_store: Arc<dyn ThreadStore> = store.clone();
    Ok((store, SubAgentGraph::new(graph_store)))
}

#[tokio::test]
async fn graph_records_child_edge_in_thread_store_lineage() -> Result<(), Box<dyn std::error::Error>>
{
    let (store, graph) = graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;

    let mut spawn = ChildAgentSpawnRecord::new("parent-thread", "child-thread", "spawn-item");
    spawn.parent_turn_id = Some("parent-turn".to_string());
    spawn.title = Some("Investigate issue".to_string());
    let summary = graph.record_child(spawn).await?;

    assert_eq!(
        summary.agent_path,
        AgentPath::for_child_thread("child-thread")
    );
    assert_eq!(summary.parent_thread_id, "parent-thread");
    assert_eq!(summary.child_thread_id, "child-thread");
    assert_eq!(summary.parent_turn_id.as_deref(), Some("parent-turn"));
    assert_eq!(summary.spawn_item_id, "spawn-item");
    assert_eq!(summary.status, ThreadStatus::Active);
    assert_eq!(summary.title.as_deref(), Some("Investigate issue"));

    let child = store.read_thread("child-thread").await?;
    let lineage = match child.lineage {
        Some(lineage) => lineage,
        None => panic!("expected child lineage"),
    };
    assert_eq!(lineage.parent_thread_id.as_deref(), Some("parent-thread"));
    assert_eq!(lineage.parent_turn_id.as_deref(), Some("parent-turn"));
    assert_eq!(lineage.parent_item_id.as_deref(), Some("spawn-item"));
    assert_eq!(lineage.fork_mode.as_deref(), Some("subagent"));
    Ok(())
}

#[tokio::test]
async fn graph_lists_direct_children_and_descendants() -> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;

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

    let direct = graph
        .list_children("parent-thread", AgentGraphListQuery::direct())
        .await?;
    assert_eq!(
        direct
            .iter()
            .map(|child| child.child_thread_id.as_str())
            .collect::<Vec<_>>(),
        vec!["child-a", "child-b"]
    );

    let descendants = graph
        .list_children("parent-thread", AgentGraphListQuery::descendants())
        .await?;
    assert_eq!(
        descendants
            .iter()
            .map(|child| child.child_thread_id.as_str())
            .collect::<Vec<_>>(),
        vec!["child-a", "child-b", "grandchild-a"]
    );
    Ok(())
}

#[tokio::test]
async fn graph_excludes_non_subagent_thread_lineage() -> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;
    store
        .create_thread(ThreadRecord::new("branch-thread"))
        .await?;
    store
        .set_lineage(ThreadLineage {
            thread_id: "branch-thread".to_string(),
            parent_thread_id: Some("parent-thread".to_string()),
            parent_turn_id: Some("parent-turn".to_string()),
            parent_item_id: Some("branch-item".to_string()),
            fork_mode: Some("branch".to_string()),
        })
        .await?;

    let children = graph
        .list_children("parent-thread", AgentGraphListQuery::direct())
        .await?;
    assert!(children.is_empty());

    let err = match graph
        .read_child(&AgentPath::for_child_thread("branch-thread"))
        .await
    {
        Ok(_) => panic!("expected non-subagent lineage to be rejected"),
        Err(err) => err,
    };
    assert!(matches!(err, SubAgentGraphError::InvalidAgentPath(_)));
    Ok(())
}

#[tokio::test]
async fn graph_rejects_duplicate_child_reparenting() -> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = graph_with_store()?;
    store.create_thread(ThreadRecord::new("parent-a")).await?;
    store.create_thread(ThreadRecord::new("parent-b")).await?;

    graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-a",
            "child-thread",
            "spawn-a",
        ))
        .await?;

    let err = match graph
        .record_child(ChildAgentSpawnRecord::new(
            "parent-b",
            "child-thread",
            "spawn-b",
        ))
        .await
    {
        Ok(_) => panic!("expected duplicate child reparenting to fail"),
        Err(err) => err,
    };
    assert!(matches!(err, SubAgentGraphError::ConflictingChildEdge(_)));

    let child = store.read_thread("child-thread").await?;
    let lineage = match child.lineage {
        Some(lineage) => lineage,
        None => panic!("expected original lineage"),
    };
    assert_eq!(lineage.parent_thread_id.as_deref(), Some("parent-a"));
    assert_eq!(lineage.parent_item_id.as_deref(), Some("spawn-a"));
    Ok(())
}

#[tokio::test]
async fn graph_rejects_invalid_child_id_before_persisting() -> Result<(), Box<dyn std::error::Error>>
{
    let (store, graph) = graph_with_store()?;
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await?;

    for invalid_child_id in ["", "child thread"] {
        let err = match graph
            .record_child(ChildAgentSpawnRecord::new(
                "parent-thread",
                invalid_child_id,
                "spawn-item",
            ))
            .await
        {
            Ok(_) => panic!("expected invalid child id to be rejected"),
            Err(err) => err,
        };
        assert!(matches!(err, SubAgentGraphError::InvalidAgentPath(_)));

        assert!(matches!(
            store.read_thread(invalid_child_id).await,
            Err(ThreadStoreError::ThreadNotFound(_))
        ));
    }

    let children = graph
        .list_children("parent-thread", AgentGraphListQuery::direct())
        .await?;
    assert!(children.is_empty());
    Ok(())
}

#[tokio::test]
async fn graph_hides_children_when_parent_is_archived_by_default()
-> Result<(), Box<dyn std::error::Error>> {
    let (store, graph) = graph_with_store()?;
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
    store
        .archive_thread("parent-thread", Some("done".to_string()))
        .await?;

    let hidden = graph
        .list_children("parent-thread", AgentGraphListQuery::direct())
        .await?;
    assert!(hidden.is_empty());

    let mut include_archived = AgentGraphListQuery::direct();
    include_archived.include_archived = true;
    let visible = graph
        .list_children("parent-thread", include_archived)
        .await?;
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].child_thread_id, "child-thread");
    Ok(())
}

#[tokio::test]
async fn graph_edges_survive_reopening_thread_store() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let db_path = temp.path().join("thread-store");
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
    }

    let reopened = Arc::new(SqliteThreadStore::open(&db_path)?);
    let graph_store: Arc<dyn ThreadStore> = reopened.clone();
    let graph = SubAgentGraph::new(graph_store);
    let children = graph
        .list_children("parent-thread", AgentGraphListQuery::direct())
        .await?;

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].agent_path.as_path_str(), "agent://child-thread");
    assert_eq!(children[0].spawn_item_id, "spawn-item");
    Ok(())
}

#[test]
fn agent_path_rejects_invalid_paths() {
    let err = match AgentPath::from_raw_path("child-thread") {
        Ok(_) => panic!("expected missing prefix to be rejected"),
        Err(err) => err,
    };
    assert!(matches!(err, SubAgentGraphError::InvalidAgentPath(_)));

    let err = match AgentPath::from_raw_path("agent://") {
        Ok(_) => panic!("expected empty child thread id to be rejected"),
        Err(err) => err,
    };
    assert!(matches!(err, SubAgentGraphError::InvalidAgentPath(_)));
}
