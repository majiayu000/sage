//! Tests for checkpoint storage implementations

use super::super::types::{Checkpoint, CheckpointType, FileSnapshot, FileState};
use super::{CheckpointStorage, CheckpointSummary, FileCheckpointStorage, MemoryCheckpointStorage};
use tempfile::TempDir;

fn create_test_checkpoint() -> Checkpoint {
    Checkpoint::new("Test checkpoint", CheckpointType::Manual).with_name("Test")
}

#[tokio::test]
async fn test_memory_storage_save_load() {
    let storage = MemoryCheckpointStorage::new();
    let checkpoint = create_test_checkpoint();

    storage.save(&checkpoint).await.unwrap();
    let loaded = storage.load(&checkpoint.id).await.unwrap();

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().description, "Test checkpoint");
}

#[tokio::test]
async fn test_memory_storage_list() {
    let storage = MemoryCheckpointStorage::new();

    let cp1 = create_test_checkpoint();
    let cp2 = create_test_checkpoint();

    storage.save(&cp1).await.unwrap();
    storage.save(&cp2).await.unwrap();

    let list = storage.list().await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_memory_storage_delete() {
    let storage = MemoryCheckpointStorage::new();
    let checkpoint = create_test_checkpoint();

    storage.save(&checkpoint).await.unwrap();
    assert!(storage.exists(&checkpoint.id).await.unwrap());

    storage.delete(&checkpoint.id).await.unwrap();
    assert!(!storage.exists(&checkpoint.id).await.unwrap());
}

#[tokio::test]
async fn test_memory_storage_content() {
    let storage = MemoryCheckpointStorage::new();
    let content = "Hello, World!";

    let content_ref = storage.store_content(content).await.unwrap();
    let loaded = storage.load_content(&content_ref).await.unwrap();

    assert_eq!(loaded, Some(content.to_string()));
}

#[tokio::test]
async fn test_file_storage_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileCheckpointStorage::new(temp_dir.path());
    let checkpoint = create_test_checkpoint();

    storage.save(&checkpoint).await.unwrap();
    let loaded = storage.load(&checkpoint.id).await.unwrap();

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().description, "Test checkpoint");
}

#[tokio::test]
async fn test_file_storage_list() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileCheckpointStorage::new(temp_dir.path());

    let cp1 = create_test_checkpoint();
    let cp2 = create_test_checkpoint();

    storage.save(&cp1).await.unwrap();
    storage.save(&cp2).await.unwrap();

    let list = storage.list().await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_file_storage_delete() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileCheckpointStorage::new(temp_dir.path());
    let checkpoint = create_test_checkpoint();

    storage.save(&checkpoint).await.unwrap();
    assert!(storage.exists(&checkpoint.id).await.unwrap());

    storage.delete(&checkpoint.id).await.unwrap();
    assert!(!storage.exists(&checkpoint.id).await.unwrap());
}

#[tokio::test]
async fn test_file_storage_content_compression() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileCheckpointStorage::new(temp_dir.path());
    let content = "Hello, World! ".repeat(1000);

    let content_ref = storage.store_content(&content).await.unwrap();
    let loaded = storage.load_content(&content_ref).await.unwrap();

    assert_eq!(loaded, Some(content));
}

#[tokio::test]
async fn test_file_storage_large_content_externalization() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileCheckpointStorage::new(temp_dir.path()).with_max_inline_size(100);

    let large_content = "x".repeat(200);

    let checkpoint =
        Checkpoint::new("With large file", CheckpointType::Auto).with_file(FileSnapshot::new(
            "large.txt",
            FileState::Exists {
                content: Some(large_content.clone()),
                content_ref: None,
            },
        ));

    storage.save(&checkpoint).await.unwrap();
    let loaded = storage.load(&checkpoint.id).await.unwrap().unwrap();

    // Content should be restored
    if let FileState::Exists { content, .. } = &loaded.files[0].state {
        assert_eq!(content.as_ref().unwrap(), &large_content);
    } else {
        panic!("Expected Exists state");
    }
}

#[tokio::test]
async fn test_file_storage_latest() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileCheckpointStorage::new(temp_dir.path());

    let cp1 = create_test_checkpoint();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let cp2 = create_test_checkpoint();

    storage.save(&cp1).await.unwrap();
    storage.save(&cp2).await.unwrap();

    let latest = storage.latest().await.unwrap().unwrap();
    assert_eq!(latest.id, cp2.id);
}

#[tokio::test]
async fn test_checkpoint_summary() {
    let checkpoint =
        Checkpoint::new("Summary test", CheckpointType::Manual).with_name("Named checkpoint");

    let summary = CheckpointSummary::from(&checkpoint);

    assert_eq!(summary.description, "Summary test");
    assert_eq!(summary.name, Some("Named checkpoint".to_string()));
    assert_eq!(summary.checkpoint_type, CheckpointType::Manual);
    assert_eq!(summary.file_count, 0);
    assert!(!summary.has_conversation);
}
