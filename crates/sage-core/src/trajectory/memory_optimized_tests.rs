//! Tests for memory-optimized trajectory recording

#[cfg(test)]
mod tests {
    use super::super::memory_optimized::{MemoryOptimizedRecorder, MemoryOptimizedConfig};
    use super::super::recorder::TrajectoryRecord;
    use crate::error::SageResult;
    use std::time::Duration;
    use tokio::fs;

    /// Create a test trajectory record
    fn create_test_record(id_suffix: u32) -> TrajectoryRecord {
        TrajectoryRecord {
            id: uuid::Uuid::new_v4(),
            task: format!("Test task {}", id_suffix),
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: chrono::Utc::now().to_rfc3339(),
            provider: "test_provider".to_string(),
            model: "test_model".to_string(),
            max_steps: 10,
            llm_interactions: vec![],
            agent_steps: vec![],
            success: true,
            final_result: Some(format!("Test result {}", id_suffix)),
            execution_time: 1.5,
        }
    }

    #[tokio::test]
    async fn test_memory_optimized_basic_operations() -> SageResult<()> {
        let temp_dir = std::env::temp_dir().join("sage_memory_test_basic");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let config = MemoryOptimizedConfig {
            max_memory_records: 5,
            max_memory_bytes: 1024 * 1024, // 1MB
            storage_dir: temp_dir.clone(),
            flush_interval: Duration::from_secs(1),
            max_record_age: Duration::from_secs(3600),
            enable_compression: false, // Disable for easier testing
            batch_size: 10,
        };

        let recorder = MemoryOptimizedRecorder::new(config).await?;

        // Test adding records
        let record1 = create_test_record(1);
        let record2 = create_test_record(2);
        let record_id1 = record1.id.clone();
        let record_id2 = record2.id.clone();

        recorder.add_record(record1).await?;
        recorder.add_record(record2).await?;

        // Test retrieving records
        let retrieved1 = recorder.get_record(&record_id1).await?;
        let retrieved2 = recorder.get_record(&record_id2).await?;

        assert!(retrieved1.is_some());
        assert!(retrieved2.is_some());
        assert_eq!(retrieved1.unwrap().task, "Test task 1");
        assert_eq!(retrieved2.unwrap().task, "Test task 2");

        // Test statistics
        let stats = recorder.statistics().await;
        assert_eq!(stats.memory_records, 2);
        assert_eq!(stats.total_records, 2);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_memory_eviction() -> SageResult<()> {
        let temp_dir = std::env::temp_dir().join("sage_memory_test_eviction");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let config = MemoryOptimizedConfig {
            max_memory_records: 3, // Small capacity to force eviction
            max_memory_bytes: 1024 * 1024,
            storage_dir: temp_dir.clone(),
            flush_interval: Duration::from_secs(10), // Long interval to test manual eviction
            max_record_age: Duration::from_secs(3600),
            enable_compression: false,
            batch_size: 10,
        };

        let recorder = MemoryOptimizedRecorder::new(config).await?;

        // Add more records than capacity
        let mut record_ids = Vec::new();
        for i in 1..=5 {
            let record = create_test_record(i);
            record_ids.push(record.id.clone());
            recorder.add_record(record).await?;
        }

        // Check statistics
        let stats = recorder.statistics().await;
        println!("Memory records: {}, Total records: {}, Evictions: {}", 
            stats.memory_records, stats.total_records, stats.memory_evictions);

        // Should have evicted some records
        assert!(stats.memory_evictions > 0);
        assert!(stats.memory_records <= 3);
        assert_eq!(stats.total_records, 5);

        // First records should be evicted from memory but available from disk
        let first_record = recorder.get_record(&record_ids[0]).await?;
        assert!(first_record.is_some()); // Should be loaded from disk

        // Recent records should still be in memory
        let recent_records = recorder.get_recent_records(2).await?;
        assert_eq!(recent_records.len(), 2);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_compression() -> SageResult<()> {
        let temp_dir = std::env::temp_dir().join("sage_memory_test_compression");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let config = MemoryOptimizedConfig {
            max_memory_records: 2,
            max_memory_bytes: 1024 * 1024,
            storage_dir: temp_dir.clone(),
            flush_interval: Duration::from_secs(10),
            max_record_age: Duration::from_secs(3600),
            enable_compression: true, // Enable compression
            batch_size: 10,
        };

        let recorder = MemoryOptimizedRecorder::new(config).await?;

        // Create a record with large content
        let mut large_record = create_test_record(1);
        large_record.final_result = Some("x".repeat(10000)); // Large content
        let record_id = large_record.id.clone();

        recorder.add_record(large_record).await?;

        // Force eviction by adding more records
        for i in 2..=4 {
            recorder.add_record(create_test_record(i)).await?;
        }

        // The large record should be compressed and saved to disk
        let retrieved = recorder.get_record(&record_id).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().final_result.unwrap().len(), 10000);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_flush_functionality() -> SageResult<()> {
        let temp_dir = std::env::temp_dir().join("sage_memory_test_flush");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let config = MemoryOptimizedConfig {
            max_memory_records: 10,
            max_memory_bytes: 1024 * 1024,
            storage_dir: temp_dir.clone(),
            flush_interval: Duration::from_secs(10),
            max_record_age: Duration::from_secs(3600),
            enable_compression: false,
            batch_size: 10,
        };

        let recorder = MemoryOptimizedRecorder::new(config).await?;

        // Add some records
        for i in 1..=3 {
            recorder.add_record(create_test_record(i)).await?;
        }

        // Manual flush
        recorder.flush().await?;

        // Check that files were created
        let mut entries = fs::read_dir(&temp_dir).await?;
        let mut file_count = 0;
        while let Some(_entry) = entries.next_entry().await? {
            file_count += 1;
        }

        // Should have created files for the records
        assert!(file_count > 0);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_memory_usage_tracking() -> SageResult<()> {
        let temp_dir = std::env::temp_dir().join("sage_memory_test_usage");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let config = MemoryOptimizedConfig {
            max_memory_records: 100,
            max_memory_bytes: 5000, // Small memory limit
            storage_dir: temp_dir.clone(),
            flush_interval: Duration::from_secs(10),
            max_record_age: Duration::from_secs(3600),
            enable_compression: false,
            batch_size: 10,
        };

        let recorder = MemoryOptimizedRecorder::new(config).await?;

        // Add records until memory limit is reached
        for i in 1..=10 {
            let mut record = create_test_record(i);
            record.final_result = Some("x".repeat(1000)); // Make records larger
            recorder.add_record(record).await?;
        }

        let stats = recorder.statistics().await;
        println!("Final memory usage: {} bytes, {} records", 
            stats.memory_bytes, stats.memory_records);

        // Should have evicted some records due to memory limit
        assert!(stats.memory_evictions > 0);
        assert!(stats.memory_bytes <= 5000 || stats.memory_records == 0);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
        Ok(())
    }
}
