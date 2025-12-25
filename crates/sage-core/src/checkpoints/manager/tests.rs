//! Tests for checkpoint manager

#[cfg(test)]
mod tests {
    use super::super::super::config::CheckpointManagerConfig;
    use super::super::super::types::{CheckpointType, RestoreOptions};
    use super::super::types::CheckpointManager;
    use tempfile::TempDir;
    use tokio::fs;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn setup_test_project() -> (TempDir, CheckpointManager) {
        let temp_dir = TempDir::new().unwrap();
        let config = CheckpointManagerConfig::new(temp_dir.path()).with_max_checkpoints(10);
        let manager = CheckpointManager::new(config);

        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).await.unwrap();

        let mut main = File::create(src_dir.join("main.rs")).await.unwrap();
        main.write_all(b"fn main() { println!(\"Hello\"); }")
            .await
            .unwrap();

        let mut lib = File::create(src_dir.join("lib.rs")).await.unwrap();
        lib.write_all(b"pub mod utils;").await.unwrap();

        (temp_dir, manager)
    }

    #[tokio::test]
    async fn test_create_full_checkpoint() {
        let (_temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("Initial checkpoint", CheckpointType::Manual)
            .await
            .unwrap();

        assert_eq!(checkpoint.description, "Initial checkpoint");
        assert_eq!(checkpoint.checkpoint_type, CheckpointType::Manual);
        assert!(checkpoint.file_count() >= 2);
    }

    #[tokio::test]
    async fn test_create_checkpoint_specific_files() {
        let (temp_dir, manager) = setup_test_project().await;
        let files = vec![temp_dir.path().join("src/main.rs")];

        let checkpoint = manager
            .create_checkpoint("Single file", CheckpointType::PreTool, files)
            .await
            .unwrap();

        assert_eq!(checkpoint.file_count(), 1);
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let (_temp_dir, manager) = setup_test_project().await;

        manager
            .create_full_checkpoint("First", CheckpointType::Manual)
            .await
            .unwrap();
        manager
            .create_full_checkpoint("Second", CheckpointType::Auto)
            .await
            .unwrap();

        let list = manager.list_checkpoints().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_restore_checkpoint() {
        let (temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("Before edit", CheckpointType::Manual)
            .await
            .unwrap();

        // Modify a file
        let main_path = temp_dir.path().join("src/main.rs");
        let mut file = File::create(&main_path).await.unwrap();
        file.write_all(b"fn main() { println!(\"Modified!\"); }")
            .await
            .unwrap();
        file.flush().await.unwrap();
        drop(file);

        // Small delay to ensure file system sync
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Restore
        let result = manager
            .restore(
                &checkpoint.id,
                RestoreOptions::files_only().without_backup(),
            )
            .await
            .unwrap();

        assert!(result.is_success());
        assert!(!result.restored_files.is_empty());

        // Small delay after restore
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let content = fs::read_to_string(&main_path).await.unwrap();
        assert!(content.contains("Hello"));
    }

    #[tokio::test]
    async fn test_should_checkpoint_for_tool() {
        let (_temp_dir, manager) = setup_test_project().await;

        assert!(manager.should_checkpoint_for_tool("Write"));
        assert!(manager.should_checkpoint_for_tool("Edit"));
        assert!(manager.should_checkpoint_for_tool("Bash"));
        assert!(!manager.should_checkpoint_for_tool("Read"));
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = CheckpointManagerConfig::new("/project")
            .with_storage_path("/custom/storage")
            .with_max_checkpoints(100)
            .without_auto_checkpoint();

        assert_eq!(
            config.storage_path,
            std::path::PathBuf::from("/custom/storage")
        );
        assert_eq!(config.max_checkpoints, 100);
        assert!(!config.auto_checkpoint_before_tools);
    }
}
