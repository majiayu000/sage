//! Tests for branching system

use super::*;
use chrono::Utc;

#[test]
fn test_branch_id() {
    let id1 = BranchId::new();
    let id2 = BranchId::new();
    assert_ne!(id1, id2);
    assert!(!id1.0.is_empty());
}

#[test]
fn test_branch_snapshot_creation() {
    let snapshot = BranchSnapshot::new("test-branch", 5)
        .with_description("Test description")
        .with_tag("important");

    assert_eq!(snapshot.name, "test-branch");
    assert_eq!(snapshot.message_index, 5);
    assert_eq!(snapshot.description, Some("Test description".to_string()));
    assert!(snapshot.tags.contains(&"important".to_string()));
    assert!(snapshot.is_root());
}

#[test]
fn test_branch_snapshot_with_parent() {
    let parent_id = BranchId::new();
    let snapshot = BranchSnapshot::new("child", 10).with_parent(parent_id.clone());

    assert!(!snapshot.is_root());
    assert_eq!(snapshot.parent_id, Some(parent_id));
}

#[tokio::test]
async fn test_branch_manager_create() {
    let manager = BranchManager::new();

    let messages = vec![SerializedMessage {
        role: "user".to_string(),
        content: "Hello".to_string(),
        name: None,
        tool_call_id: None,
        timestamp: Utc::now(),
    }];

    let branch_id = manager.create_branch(Some("test"), messages, vec![]).await;

    assert_eq!(manager.count().await, 1);
    assert!(manager.get(&branch_id).await.is_some());
}

#[tokio::test]
async fn test_branch_manager_auto_name() {
    let manager = BranchManager::new();

    let id1 = manager.create_branch(None, vec![], vec![]).await;
    let id2 = manager.create_branch(None, vec![], vec![]).await;

    let branch1 = manager.get(&id1).await.unwrap();
    let branch2 = manager.get(&id2).await.unwrap();

    assert!(branch1.name.contains("branch-"));
    assert!(branch2.name.contains("branch-"));
    assert_ne!(branch1.name, branch2.name);
}

#[tokio::test]
async fn test_branch_manager_switch() {
    let manager = BranchManager::new();

    let id1 = manager.create_branch(Some("first"), vec![], vec![]).await;
    let id2 = manager.create_branch(Some("second"), vec![], vec![]).await;

    let current = manager.current().await.unwrap();
    assert_eq!(current.id, id2);

    manager.switch_to(&id1).await;
    let current = manager.current().await.unwrap();
    assert_eq!(current.id, id1);
}

#[tokio::test]
async fn test_branch_manager_delete() {
    let manager = BranchManager::new();

    let id = manager
        .create_branch(Some("to-delete"), vec![], vec![])
        .await;
    assert_eq!(manager.count().await, 1);

    let deleted = manager.delete(&id).await;
    assert!(deleted.is_some());
    assert_eq!(manager.count().await, 0);
}

#[tokio::test]
async fn test_branch_manager_rename() {
    let manager = BranchManager::new();

    let id = manager
        .create_branch(Some("old-name"), vec![], vec![])
        .await;
    manager.rename(&id, "new-name").await;

    let branch = manager.get(&id).await.unwrap();
    assert_eq!(branch.name, "new-name");
}

#[tokio::test]
async fn test_branch_manager_tags() {
    let manager = BranchManager::new();

    let id = manager.create_branch(Some("tagged"), vec![], vec![]).await;
    manager.add_tag(&id, "important").await;
    manager.add_tag(&id, "wip").await;

    let tagged = manager.list_by_tag("important").await;
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].id, id);
}

#[tokio::test]
async fn test_branch_manager_ancestry() {
    let manager = BranchManager::new();

    let id1 = manager.create_branch(Some("root"), vec![], vec![]).await;
    let id2 = manager.create_branch(Some("child"), vec![], vec![]).await;
    let id3 = manager
        .create_branch(Some("grandchild"), vec![], vec![])
        .await;

    let ancestry = manager.get_ancestry(&id3).await;
    assert_eq!(ancestry.len(), 3);
    assert_eq!(ancestry[0].id, id1);
    assert_eq!(ancestry[1].id, id2);
    assert_eq!(ancestry[2].id, id3);
}

#[tokio::test]
async fn test_branch_manager_tree() {
    let manager = BranchManager::new();

    manager.create_branch(Some("root"), vec![], vec![]).await;
    manager.create_branch(Some("child1"), vec![], vec![]).await;

    // Switch back to root and create another child
    let branches = manager.list().await;
    let root = branches.iter().find(|b| b.name == "root").unwrap();
    manager.switch_to(&root.id).await;
    manager.create_branch(Some("child2"), vec![], vec![]).await;

    let tree = manager.get_tree().await;
    assert!(tree.len() >= 3);
}

#[tokio::test]
async fn test_branch_manager_max_branches() {
    let manager = BranchManager::new().with_max_branches(3);

    for i in 0..5 {
        manager
            .create_branch(Some(&format!("branch-{}", i)), vec![], vec![])
            .await;
    }

    // Should only keep max_branches
    assert!(manager.count().await <= 3);
}

#[tokio::test]
async fn test_branch_manager_clear() {
    let manager = BranchManager::new();

    manager.create_branch(Some("a"), vec![], vec![]).await;
    manager.create_branch(Some("b"), vec![], vec![]).await;

    assert_eq!(manager.count().await, 2);

    manager.clear().await;

    assert!(manager.is_empty().await);
    assert!(manager.current().await.is_none());
}

#[tokio::test]
async fn test_branch_manager_export_import() {
    let manager = BranchManager::new();

    manager.create_branch(Some("test"), vec![], vec![]).await;

    let exported = manager.export().await;

    let manager2 = BranchManager::new();
    let count = manager2.import(&exported).await.unwrap();

    assert_eq!(count, 1);
    assert_eq!(manager2.count().await, 1);
}

#[tokio::test]
async fn test_branch_merge() {
    let manager = BranchManager::new();

    let msg1 = SerializedMessage {
        role: "user".to_string(),
        content: "First".to_string(),
        name: None,
        tool_call_id: None,
        timestamp: Utc::now(),
    };
    let msg2 = SerializedMessage {
        role: "user".to_string(),
        content: "Second".to_string(),
        name: None,
        tool_call_id: None,
        timestamp: Utc::now(),
    };

    let id1 = manager
        .create_branch(Some("branch1"), vec![msg1], vec![])
        .await;

    // Create independent branch
    *manager.current_branch.write().await = None;
    let id2 = manager
        .create_branch(Some("branch2"), vec![msg2], vec![])
        .await;

    let merged_id = manager.merge(&id1, &id2).await.unwrap();
    let merged = manager.get(&merged_id).await.unwrap();

    assert_eq!(merged.messages.len(), 2);
    assert!(merged.tags.contains(&"merge".to_string()));
}

#[test]
fn test_serialized_message() {
    let msg = SerializedMessage {
        role: "assistant".to_string(),
        content: "Hello!".to_string(),
        name: Some("Claude".to_string()),
        tool_call_id: None,
        timestamp: Utc::now(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: SerializedMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.role, "assistant");
    assert_eq!(parsed.content, "Hello!");
}

#[test]
fn test_serialized_tool_call() {
    let call = SerializedToolCall {
        tool_name: "Read".to_string(),
        arguments: serde_json::json!({"path": "/test"}),
        result: Some("content".to_string()),
        success: true,
        timestamp: Utc::now(),
    };

    let json = serde_json::to_string(&call).unwrap();
    let parsed: SerializedToolCall = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.tool_name, "Read");
    assert!(parsed.success);
}
