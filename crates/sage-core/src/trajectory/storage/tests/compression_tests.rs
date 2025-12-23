//! Compression-related tests for trajectory storage

use super::super::*;
use super::helpers::create_test_record;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_save_and_load_compressed() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().join("test_trajectory.json.gz");
    let storage = FileStorage::new(&storage_path).unwrap();

    // Create and save a compressed trajectory
    let record = create_test_record();
    let record_id = record.id;

    storage.save_compressed(&record).await.unwrap();

    // Verify file exists and is compressed
    assert!(storage_path.exists());
    assert!(FileStorage::is_compressed_file(&storage_path));

    // Load the compressed trajectory
    let loaded = storage.load_compressed(record_id).await.unwrap();
    assert!(loaded.is_some());

    let loaded_record = loaded.unwrap();
    assert_eq!(loaded_record.id, record_id);
    assert_eq!(loaded_record.task, "Test task");
    assert_eq!(loaded_record.success, true);
    assert_eq!(loaded_record.agent_steps.len(), 1);
}

#[tokio::test]
async fn test_load_compressed_with_auto_detection() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save compressed trajectory
    let record = create_test_record();
    let record_id = record.id;

    storage.save_compressed(&record).await.unwrap();

    // load_compressed should automatically detect the .json.gz file
    let loaded = storage.load_compressed(record_id).await.unwrap();
    assert!(loaded.is_some());

    let loaded_record = loaded.unwrap();
    assert_eq!(loaded_record.id, record_id);
}

#[tokio::test]
async fn test_load_compressed_fallback_to_uncompressed() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save uncompressed trajectory using regular save
    let record = create_test_record();
    let record_id = record.id;

    storage.save(&record).await.unwrap();

    // load_compressed should fall back to reading the uncompressed .json file
    let loaded = storage.load_compressed(record_id).await.unwrap();
    assert!(loaded.is_some());

    let loaded_record = loaded.unwrap();
    assert_eq!(loaded_record.id, record_id);
}

#[tokio::test]
async fn test_compression_reduces_file_size() {
    let temp_dir = TempDir::new().unwrap();

    // Create uncompressed file
    let uncompressed_path = temp_dir.path().join("uncompressed.json");
    let storage_uncompressed = FileStorage::new(&uncompressed_path).unwrap();

    // Create compressed file
    let compressed_path = temp_dir.path().join("compressed.json.gz");
    let storage_compressed = FileStorage::new(&compressed_path).unwrap();

    // Save the same record in both formats
    let record = create_test_record();
    storage_uncompressed.save(&record).await.unwrap();
    storage_compressed.save_compressed(&record).await.unwrap();

    // Check file sizes
    let uncompressed_size = fs::metadata(&uncompressed_path).await.unwrap().len();
    let compressed_size = fs::metadata(&compressed_path).await.unwrap().len();

    // Compressed should be smaller (with reasonable test data, typically 5-10x smaller)
    assert!(compressed_size < uncompressed_size);
    println!(
        "Uncompressed: {} bytes, Compressed: {} bytes, Ratio: {:.2}x",
        uncompressed_size,
        compressed_size,
        uncompressed_size as f64 / compressed_size as f64
    );
}

#[tokio::test]
async fn test_list_includes_compressed_files() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save one compressed and one uncompressed trajectory
    let record1 = create_test_record();
    let record2 = create_test_record();

    storage.save_compressed(&record1).await.unwrap();
    storage.save(&record2).await.unwrap();

    // List should include both files
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&record1.id));
    assert!(ids.contains(&record2.id));
}

#[tokio::test]
async fn test_delete_compressed_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save compressed trajectory
    let record = create_test_record();
    let record_id = record.id;

    storage.save_compressed(&record).await.unwrap();

    // Verify file exists
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 1);

    // Delete the trajectory
    storage.delete(record_id).await.unwrap();

    // Verify file is deleted
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 0);
}

#[tokio::test]
async fn test_statistics_includes_compressed_files() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save compressed and uncompressed trajectories
    let record1 = create_test_record();
    let record2 = create_test_record();

    storage.save_compressed(&record1).await.unwrap();
    storage.save(&record2).await.unwrap();

    // Get statistics
    let stats = storage.statistics().await.unwrap();

    assert_eq!(stats.total_records, 2);
    assert!(stats.total_size_bytes > 0);
    assert!(stats.average_record_size > 0);
}

#[tokio::test]
async fn test_is_compressed_file() {
    assert!(FileStorage::is_compressed_file(Path::new("file.json.gz")));
    assert!(FileStorage::is_compressed_file(Path::new("file.gz")));
    assert!(!FileStorage::is_compressed_file(Path::new("file.json")));
    assert!(!FileStorage::is_compressed_file(Path::new("file.txt")));
    assert!(!FileStorage::is_compressed_file(Path::new("file")));
}

#[tokio::test]
async fn test_load_nonexistent_compressed_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Try to load a non-existent trajectory
    let fake_id = uuid::Uuid::new_v4();
    let result = storage.load_compressed(fake_id).await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_with_compression_config_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with compression enabled
    let storage = FileStorage::with_compression(&storage_dir, true).unwrap();

    // Save a record using the trait's save() method
    let record = create_test_record();
    let record_id = record.id;

    storage.save(&record).await.unwrap();

    // Verify that a compressed file was created
    let mut entries = fs::read_dir(&storage_dir).await.unwrap();
    let mut found_gz = false;

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            found_gz = true;
            break;
        }
    }

    assert!(
        found_gz,
        "Expected to find a .gz file when compression is enabled"
    );

    // Load the record back
    let loaded = storage.load(record_id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().id, record_id);
}

#[tokio::test]
async fn test_with_compression_config_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with compression disabled
    let storage = FileStorage::with_compression(&storage_dir, false).unwrap();

    // Save a record using the trait's save() method
    let record = create_test_record();
    let record_id = record.id;

    storage.save(&record).await.unwrap();

    // Verify that an uncompressed JSON file was created (not .gz)
    let mut entries = fs::read_dir(&storage_dir).await.unwrap();
    let mut found_json = false;

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext == Some("json") && !path.to_str().unwrap().ends_with(".gz") {
            found_json = true;
            break;
        }
    }

    assert!(
        found_json,
        "Expected to find a .json file when compression is disabled"
    );

    // Load the record back
    let loaded = storage.load(record_id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().id, record_id);
}

#[tokio::test]
async fn test_new_defaults_to_no_compression() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();

    // Create storage with new() which should default to no compression
    let storage = FileStorage::new(&storage_dir).unwrap();

    // Save a record
    let record = create_test_record();

    storage.save(&record).await.unwrap();

    // Verify that an uncompressed JSON file was created
    let mut entries = fs::read_dir(&storage_dir).await.unwrap();
    let mut found_json = false;

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext == Some("json") && !path.to_str().unwrap().ends_with(".gz") {
            found_json = true;
            break;
        }
    }

    assert!(found_json, "Expected new() to default to no compression");
}
