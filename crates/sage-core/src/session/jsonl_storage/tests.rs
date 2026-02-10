//! Tests for JSONL storage

use std::path::PathBuf;
use tempfile::TempDir;

use super::super::types::{SessionContext};
use crate::session::types::unified::SessionMessage;
use super::storage::JsonlSessionStorage;
use super::tracker::MessageChainTracker;

#[tokio::test]
async fn test_create_session() {
    let tmp = TempDir::new().unwrap();
    let storage = JsonlSessionStorage::new(tmp.path());

    let metadata = storage
        .create_session("test-session", PathBuf::from("/tmp"))
        .await
        .unwrap();

    assert_eq!(metadata.id, "test-session");
    assert!(storage.session_exists(&"test-session".to_string()).await);
}

#[tokio::test]
async fn test_append_and_load_messages() {
    let tmp = TempDir::new().unwrap();
    let storage = JsonlSessionStorage::new(tmp.path());
    let session_id = "test-session".to_string();

    storage
        .create_session(&session_id, PathBuf::from("/tmp"))
        .await
        .unwrap();

    let context = SessionContext::new(PathBuf::from("/tmp"));
    let msg1 = SessionMessage::user("Hello", &session_id, context.clone());
    let msg2 = SessionMessage::assistant("Hi!", &session_id, context, Some(msg1.uuid.clone()));

    storage.append_message(&session_id, &msg1).await.unwrap();
    storage.append_message(&session_id, &msg2).await.unwrap();

    let messages = storage.load_messages(&session_id).await.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].uuid, msg1.uuid);
    assert_eq!(messages[1].uuid, msg2.uuid);
}

#[tokio::test]
async fn test_message_chain() {
    let tmp = TempDir::new().unwrap();
    let storage = JsonlSessionStorage::new(tmp.path());
    let session_id = "test-session".to_string();

    storage
        .create_session(&session_id, PathBuf::from("/tmp"))
        .await
        .unwrap();

    let context = SessionContext::new(PathBuf::from("/tmp"));
    let msg1 = SessionMessage::user("First", &session_id, context.clone());
    let msg2 = SessionMessage::assistant(
        "Second",
        &session_id,
        context.clone(),
        Some(msg1.uuid.clone()),
    );
    let msg3 = SessionMessage::user("Third", &session_id, context).with_parent(&msg2.uuid);

    storage.append_message(&session_id, &msg1).await.unwrap();
    storage.append_message(&session_id, &msg2).await.unwrap();
    storage.append_message(&session_id, &msg3).await.unwrap();

    let chain = storage
        .get_message_chain(&session_id, &msg3.uuid)
        .await
        .unwrap();
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].uuid, msg1.uuid);
    assert_eq!(chain[1].uuid, msg2.uuid);
    assert_eq!(chain[2].uuid, msg3.uuid);
}

#[tokio::test]
async fn test_message_chain_tracker() {
    let context = SessionContext::new(PathBuf::from("/tmp"));
    let mut tracker = MessageChainTracker::new()
        .with_session("test")
        .with_context(context);

    let msg1 = tracker.create_user_message("Hello");
    assert!(msg1.parent_uuid.is_none());

    let msg2 = tracker.create_assistant_message("Hi!");
    assert_eq!(msg2.parent_uuid, Some(msg1.uuid.clone()));

    let msg3 = tracker.create_user_message("How are you?");
    assert_eq!(msg3.parent_uuid, Some(msg2.uuid.clone()));
}

#[tokio::test]
async fn test_delete_session() {
    let tmp = TempDir::new().unwrap();
    let storage = JsonlSessionStorage::new(tmp.path());
    let session_id = "test-session".to_string();

    storage
        .create_session(&session_id, PathBuf::from("/tmp"))
        .await
        .unwrap();

    assert!(storage.session_exists(&session_id).await);

    storage.delete_session(&session_id).await.unwrap();
    assert!(!storage.session_exists(&session_id).await);
}

#[tokio::test]
async fn test_list_sessions() {
    let tmp = TempDir::new().unwrap();
    let storage = JsonlSessionStorage::new(tmp.path());

    storage
        .create_session("session-1", PathBuf::from("/tmp/1"))
        .await
        .unwrap();
    storage
        .create_session("session-2", PathBuf::from("/tmp/2"))
        .await
        .unwrap();

    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2);
}
