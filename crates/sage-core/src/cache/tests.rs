//! Cache system tests

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::llm::{LLMMessage, LLMResponse, MessageRole};
    use crate::types::LLMUsage;
    use std::time::Duration;
    use tokio::fs;

    #[tokio::test]
    async fn test_memory_cache_basic_operations() {
        let storage = MemoryStorage::new(10);

        let key = CacheKey::new("test", "key1");
        let entry = CacheEntry::new(
            serde_json::json!({"test": "data"}),
            Some(Duration::from_secs(60)),
        );

        // Test set and get
        storage.set(key.clone(), entry.clone()).await.unwrap();
        let retrieved = storage.get(&key).await.unwrap();
        assert!(retrieved.is_some());

        // Test remove
        storage.remove(&key).await.unwrap();
        let retrieved = storage.get(&key).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_llm_cache_operations() {
        let cache_config = CacheConfig {
            enable_memory_cache: true,
            memory_capacity: 10,
            enable_disk_cache: false,
            ..Default::default()
        };

        let cache_manager = CacheManager::new(cache_config).unwrap();
        let llm_cache = LLMCache::new(cache_manager, Some(Duration::from_secs(60)));

        let messages = vec![LLMMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: std::collections::HashMap::new(),
        }];

        let response = LLMResponse {
            content: "Test response".to_string(),
            tool_calls: Vec::new(),
            usage: Some(LLMUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                cost_usd: Some(0.001),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            }),
            model: Some("test-model".to_string()),
            finish_reason: Some("stop".to_string()),
            id: None,
            metadata: std::collections::HashMap::new(),
        };

        // Test cache miss
        let cached = llm_cache
            .get_response("test", "model", &messages, None)
            .await
            .unwrap();
        assert!(cached.is_none());

        // Test cache set
        llm_cache
            .cache_response("test", "model", &messages, None, &response, None)
            .await
            .unwrap();

        // Test cache hit
        let cached = llm_cache
            .get_response("test", "model", &messages, None)
            .await
            .unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Test response");
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let storage = MemoryStorage::new(10);

        let key = CacheKey::new("test", "expiring_key");
        let entry = CacheEntry::new(
            serde_json::json!({"test": "data"}),
            Some(Duration::from_millis(100)), // Very short TTL
        );

        // Set entry
        storage.set(key.clone(), entry).await.unwrap();

        // Should be available immediately
        let retrieved = storage.get(&key).await.unwrap();
        assert!(retrieved.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired and removed
        let retrieved = storage.get(&key).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let storage = MemoryStorage::new(10);

        let key1 = CacheKey::new("test", "key1");
        let key2 = CacheKey::new("test", "key2");
        let entry = CacheEntry::new(
            serde_json::json!({"test": "data"}),
            Some(Duration::from_secs(60)),
        );

        // Add entries
        storage.set(key1.clone(), entry.clone()).await.unwrap();
        storage.set(key2.clone(), entry.clone()).await.unwrap();

        // Generate some hits and misses
        let _ = storage.get(&key1).await.unwrap(); // hit
        let _ = storage.get(&key2).await.unwrap(); // hit
        let _ = storage
            .get(&CacheKey::new("test", "nonexistent"))
            .await
            .unwrap(); // miss

        let stats = storage.statistics().await.unwrap();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!(stats.size_bytes > 0);
    }

    #[tokio::test]
    async fn test_disk_storage_basic() {
        let temp_dir = std::env::temp_dir().join("sage_cache_test");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let storage = DiskStorage::new(&temp_dir, 1024 * 1024).unwrap(); // 1MB capacity

        let key = CacheKey::new("test", "disk_key");
        let entry = CacheEntry::new(
            serde_json::json!({"test": "disk_data"}),
            Some(Duration::from_secs(60)),
        );

        // Test set and get
        storage.set(key.clone(), entry.clone()).await.unwrap();
        let retrieved = storage.get(&key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().data,
            serde_json::json!({"test": "disk_data"})
        );

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let _messages = vec![LLMMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: std::collections::HashMap::new(),
        }];

        let key1 = CacheKey::llm_response("openai", "gpt-4", 12345, Some(67890));
        let key2 = CacheKey::llm_response("openai", "gpt-4", 12345, Some(67890));
        let key3 = CacheKey::llm_response("openai", "gpt-4", 12345, Some(99999));

        // Same parameters should generate same key
        assert_eq!(key1.hash, key2.hash);

        // Different parameters should generate different keys
        assert_ne!(key1.hash, key3.hash);
    }

    #[tokio::test]
    async fn test_cache_manager_multi_layer() {
        let cache_config = CacheConfig {
            enable_memory_cache: true,
            memory_capacity: 5,
            enable_disk_cache: true,
            disk_cache_dir: std::env::temp_dir()
                .join("sage_cache_test_multi")
                .to_string_lossy()
                .to_string(),
            disk_capacity: 1024 * 1024,
            ..Default::default()
        };

        let cache_manager = CacheManager::new(cache_config).unwrap();

        let key = CacheKey::new("test", "multi_layer");
        let test_data = "multi layer test data";

        // Set data
        cache_manager
            .set(key.clone(), test_data, Some(Duration::from_secs(60)))
            .await
            .unwrap();

        // Get data (should hit memory cache)
        let retrieved: Option<String> = cache_manager.get(&key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_data);

        // Get statistics
        let stats = cache_manager.statistics().await.unwrap();
        assert!(stats.total_hits > 0);

        // Cleanup
        let _ = fs::remove_dir_all(std::env::temp_dir().join("sage_cache_test_multi")).await;
    }
}
