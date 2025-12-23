//! Rotation-related tests for trajectory storage

use super::super::*;
use super::helpers::create_test_record;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_rotation_max_trajectories() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with max 3 trajectories
    let rotation = RotationConfig::with_max_trajectories(3);
    let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

    // Save 5 trajectories
    for _ in 0..5 {
        let record = create_test_record();
        storage.save(&record).await.unwrap();
        // Small delay to ensure different modification times
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Should only have 3 files
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 3, "Should only keep 3 trajectories");
}

#[tokio::test]
async fn test_rotation_total_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // First, create a test file to determine approximate size
    let test_storage = FileStorage::new(&storage_dir).unwrap();
    let test_record = create_test_record();
    test_storage.save(&test_record).await.unwrap();

    let stats = test_storage.statistics().await.unwrap();
    let file_size = stats.average_record_size;

    // Clean up test file
    test_storage.delete(test_record.id).await.unwrap();

    // Create storage with size limit for ~2.5 files
    let size_limit = file_size * 5 / 2;
    let rotation = RotationConfig::with_total_size_limit(size_limit);
    let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

    // Save 5 trajectories
    for _ in 0..5 {
        let record = create_test_record();
        storage.save(&record).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Should keep only 2 files (since we set limit to ~2.5 files)
    let stats = storage.statistics().await.unwrap();
    assert!(
        stats.total_records <= 2,
        "Should keep at most 2 trajectories based on size limit"
    );
    assert!(
        stats.total_size_bytes <= size_limit,
        "Total size should be within limit"
    );
}

#[tokio::test]
async fn test_rotation_with_both_limits() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with both limits
    let rotation = RotationConfig::with_limits(5, 1024 * 1024); // 5 files, 1MB
    let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

    // Save 10 trajectories
    for _ in 0..10 {
        let record = create_test_record();
        storage.save(&record).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Should only have 5 files (limited by max_trajectories)
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 5, "Should only keep 5 trajectories");
}

#[tokio::test]
async fn test_rotation_with_compression() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with compression and max 3 trajectories
    let rotation = RotationConfig::with_max_trajectories(3);
    let storage = FileStorage::with_config(&storage_dir, true, rotation).unwrap();

    // Save 5 trajectories
    for _ in 0..5 {
        let record = create_test_record();
        storage.save(&record).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Should only have 3 compressed files
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 3, "Should only keep 3 compressed trajectories");

    // Verify files are compressed
    let mut entries = fs::read_dir(&storage_dir).await.unwrap();
    let mut gz_count = 0;

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            gz_count += 1;
        }
    }

    assert_eq!(gz_count, 3, "Should have 3 .gz files");
}

#[tokio::test]
async fn test_rotation_keeps_newest_files() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with max 2 trajectories
    let rotation = RotationConfig::with_max_trajectories(2);
    let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

    // Save 3 trajectories and remember their IDs
    let mut ids = Vec::new();
    for _ in 0..3 {
        let record = create_test_record();
        ids.push(record.id);
        storage.save(&record).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Should only have the last 2 trajectories
    let remaining_ids = storage.list().await.unwrap();
    assert_eq!(remaining_ids.len(), 2);

    // The first ID should be deleted, last 2 should remain
    assert!(!remaining_ids.contains(&ids[0]), "Oldest should be deleted");
    assert!(remaining_ids.contains(&ids[1]), "Second should remain");
    assert!(remaining_ids.contains(&ids[2]), "Newest should remain");
}

#[tokio::test]
async fn test_rotation_no_limits() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage without rotation limits
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save 5 trajectories
    for _ in 0..5 {
        let record = create_test_record();
        storage.save(&record).await.unwrap();
        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    }

    // Should keep all 5 files
    let ids = storage.list().await.unwrap();
    assert_eq!(
        ids.len(),
        5,
        "Should keep all trajectories when no limits set"
    );
}

#[tokio::test]
async fn test_rotation_config_builders() {
    // Test with_max_trajectories
    let config1 = RotationConfig::with_max_trajectories(10);
    assert_eq!(config1.max_trajectories, Some(10));
    assert_eq!(config1.total_size_limit, None);

    // Test with_total_size_limit
    let config2 = RotationConfig::with_total_size_limit(1024);
    assert_eq!(config2.max_trajectories, None);
    assert_eq!(config2.total_size_limit, Some(1024));

    // Test with_limits
    let config3 = RotationConfig::with_limits(5, 2048);
    assert_eq!(config3.max_trajectories, Some(5));
    assert_eq!(config3.total_size_limit, Some(2048));

    // Test default
    let config4 = RotationConfig::default();
    assert_eq!(config4.max_trajectories, None);
    assert_eq!(config4.total_size_limit, None);
}

#[tokio::test]
async fn test_rotation_does_not_affect_file_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("single_trajectory.json");

    // Create storage pointing to a single file with rotation config
    let rotation = RotationConfig::with_max_trajectories(1);
    let storage = FileStorage::with_config(&file_path, false, rotation).unwrap();

    // Save multiple times to the same file
    for _ in 0..3 {
        let record = create_test_record();
        storage.save(&record).await.unwrap();
    }

    // File should still exist (rotation shouldn't delete single file mode)
    assert!(file_path.exists());

    // Should only have 1 record (the file gets overwritten)
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 1);
}
