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

    let save_result = storage.save(&checkpoint).await;
    assert!(save_result.is_ok());
    let loaded_result = storage.load(&checkpoint.id).await;
    assert!(loaded_result.is_ok());
    if let Ok(loaded) = loaded_result {
        assert!(loaded.is_some());
        if let Some(loaded) = loaded {
            assert_eq!(loaded.description, "Test checkpoint");
        }
    }
}

#[tokio::test]
async fn test_memory_storage_list() {
    let storage = MemoryCheckpointStorage::new();

    let cp1 = create_test_checkpoint();
    let cp2 = create_test_checkpoint();

    assert!(storage.save(&cp1).await.is_ok());
    assert!(storage.save(&cp2).await.is_ok());

    let list_result = storage.list().await;
    assert!(list_result.is_ok());
    if let Ok(list) = list_result {
        assert_eq!(list.len(), 2);
    }
}

#[tokio::test]
async fn test_memory_storage_delete() {
    let storage = MemoryCheckpointStorage::new();
    let checkpoint = create_test_checkpoint();

    assert!(storage.save(&checkpoint).await.is_ok());
    let exists_before_result = storage.exists(&checkpoint.id).await;
    assert!(exists_before_result.is_ok());
    if let Ok(exists_before) = exists_before_result {
        assert!(exists_before);
    }

    assert!(storage.delete(&checkpoint.id).await.is_ok());
    let exists_after_result = storage.exists(&checkpoint.id).await;
    assert!(exists_after_result.is_ok());
    if let Ok(exists_after) = exists_after_result {
        assert!(!exists_after);
    }
}

#[tokio::test]
async fn test_memory_storage_content() {
    let storage = MemoryCheckpointStorage::new();
    let content = "Hello, World!";

    let content_ref_result = storage.store_content(content).await;
    assert!(content_ref_result.is_ok());
    if let Ok(content_ref) = content_ref_result {
        let loaded_result = storage.load_content(&content_ref).await;
        assert!(loaded_result.is_ok());
        if let Ok(loaded) = loaded_result {
            assert_eq!(loaded, Some(content.to_string()));
        }
    }
}

#[tokio::test]
async fn test_file_storage_save_load() {
    let temp_dir = TempDir::new();
    assert!(temp_dir.is_ok());
    if let Ok(temp_dir) = temp_dir {
        let storage = FileCheckpointStorage::new(temp_dir.path());
        let checkpoint = create_test_checkpoint();

        assert!(storage.save(&checkpoint).await.is_ok());
        let loaded_result = storage.load(&checkpoint.id).await;
        assert!(loaded_result.is_ok());
        if let Ok(loaded) = loaded_result {
            assert!(loaded.is_some());
            if let Some(loaded) = loaded {
                assert_eq!(loaded.description, "Test checkpoint");
            }
        }
    }
}

#[tokio::test]
async fn test_file_storage_list() {
    let temp_dir = TempDir::new();
    assert!(temp_dir.is_ok());
    if let Ok(temp_dir) = temp_dir {
        let storage = FileCheckpointStorage::new(temp_dir.path());

        let cp1 = create_test_checkpoint();
        let cp2 = create_test_checkpoint();

        assert!(storage.save(&cp1).await.is_ok());
        assert!(storage.save(&cp2).await.is_ok());

        let list_result = storage.list().await;
        assert!(list_result.is_ok());
        if let Ok(list) = list_result {
            assert_eq!(list.len(), 2);
        }
    }
}

#[tokio::test]
async fn test_file_storage_delete() {
    let temp_dir = TempDir::new();
    assert!(temp_dir.is_ok());
    if let Ok(temp_dir) = temp_dir {
        let storage = FileCheckpointStorage::new(temp_dir.path());
        let checkpoint = create_test_checkpoint();

        assert!(storage.save(&checkpoint).await.is_ok());
        let exists_before_result = storage.exists(&checkpoint.id).await;
        assert!(exists_before_result.is_ok());
        if let Ok(exists_before) = exists_before_result {
            assert!(exists_before);
        }

        assert!(storage.delete(&checkpoint.id).await.is_ok());
        let exists_after_result = storage.exists(&checkpoint.id).await;
        assert!(exists_after_result.is_ok());
        if let Ok(exists_after) = exists_after_result {
            assert!(!exists_after);
        }
    }
}

#[tokio::test]
async fn test_file_storage_content_compression() {
    let temp_dir = TempDir::new();
    assert!(temp_dir.is_ok());
    if let Ok(temp_dir) = temp_dir {
        let storage = FileCheckpointStorage::new(temp_dir.path());
        let content = "Hello, World! ".repeat(1000);

        let content_ref_result = storage.store_content(&content).await;
        assert!(content_ref_result.is_ok());
        if let Ok(content_ref) = content_ref_result {
            let loaded_result = storage.load_content(&content_ref).await;
            assert!(loaded_result.is_ok());
            if let Ok(loaded) = loaded_result {
                assert_eq!(loaded, Some(content));
            }
        }
    }
}

#[tokio::test]
async fn test_file_storage_large_content_externalization() {
    let temp_dir = TempDir::new();
    assert!(temp_dir.is_ok());
    if let Ok(temp_dir) = temp_dir {
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

        assert!(storage.save(&checkpoint).await.is_ok());
        let loaded_result = storage.load(&checkpoint.id).await;
        assert!(loaded_result.is_ok());
        if let Ok(loaded) = loaded_result {
            assert!(loaded.is_some());
            if let Some(loaded) = loaded {
                // Content should be restored
                if let FileState::Exists { content, .. } = &loaded.files[0].state {
                    assert!(content.is_some());
                    if let Some(content) = content {
                        assert_eq!(content, &large_content);
                    }
                } else {
                    panic!("Expected Exists state");
                }
            }
        }
    }
}

#[tokio::test]
async fn test_file_storage_latest() {
    let temp_dir = TempDir::new();
    assert!(temp_dir.is_ok());
    if let Ok(temp_dir) = temp_dir {
        let storage = FileCheckpointStorage::new(temp_dir.path());

        let cp1 = create_test_checkpoint();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let cp2 = create_test_checkpoint();

        assert!(storage.save(&cp1).await.is_ok());
        assert!(storage.save(&cp2).await.is_ok());

        let latest_result = storage.latest().await;
        assert!(latest_result.is_ok());
        if let Ok(latest) = latest_result {
            assert!(latest.is_some());
            if let Some(latest) = latest {
                assert_eq!(latest.id, cp2.id);
            }
        }
    }
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
