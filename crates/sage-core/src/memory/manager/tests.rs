//! Tests for memory manager

#[cfg(test)]
mod manager_tests {
    use super::super::helpers::calculate_similarity;
    use crate::memory::manager::{create_memory_manager, MemoryConfig, MemoryManager};
    use crate::memory::types::{Memory, MemorySource, MemoryType};

    #[tokio::test]
    async fn test_manager_creation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config).await.unwrap();
        assert_eq!(manager.storage.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        let memory = Memory::fact("Test fact");
        let id = manager.store(memory).await.unwrap();

        let retrieved = manager.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test fact");
    }

    #[tokio::test]
    async fn test_remember_fact() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Rust uses Cargo", MemorySource::Agent)
            .await
            .unwrap();

        let facts = manager.facts().await.unwrap();
        assert_eq!(facts.len(), 1);
        assert!(facts[0].content.contains("Cargo"));
    }

    #[tokio::test]
    async fn test_remember_preference() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_preference("User prefers dark mode")
            .await
            .unwrap();

        let prefs = manager.preferences().await.unwrap();
        assert_eq!(prefs.len(), 1);
        assert!(prefs[0].metadata.pinned); // Preferences are pinned by default
    }

    #[tokio::test]
    async fn test_remember_lesson() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_lesson("Always check return values")
            .await
            .unwrap();

        let lessons = manager.lessons().await.unwrap();
        assert_eq!(lessons.len(), 1);
    }

    #[tokio::test]
    async fn test_search() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Rust is a systems language", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Python is interpreted", MemorySource::User)
            .await
            .unwrap();

        let results = manager.find("Rust").await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_find_by_type() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("A fact", MemorySource::User)
            .await
            .unwrap();
        manager.remember_preference("A preference").await.unwrap();

        let facts = manager.find_by_type(MemoryType::Fact).await.unwrap();
        assert_eq!(facts.len(), 1);

        let prefs = manager.find_by_type(MemoryType::Preference).await.unwrap();
        assert_eq!(prefs.len(), 1);
    }

    #[tokio::test]
    async fn test_pin_unpin() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        let id = manager
            .remember_fact("Test", MemorySource::User)
            .await
            .unwrap();

        manager.pin(&id).await.unwrap();
        let pinned = manager.pinned().await.unwrap();
        assert_eq!(pinned.len(), 1);

        manager.unpin(&id).await.unwrap();
        let pinned = manager.pinned().await.unwrap();
        assert_eq!(pinned.len(), 0);
    }

    #[tokio::test]
    async fn test_delete() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        let id = manager
            .remember_fact("To delete", MemorySource::User)
            .await
            .unwrap();
        manager.delete(&id).await.unwrap();

        assert!(manager.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Fact 1", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Fact 2", MemorySource::User)
            .await
            .unwrap();
        manager.remember_preference("Pref 1").await.unwrap();

        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.by_type.get("Fact"), Some(&2));
        assert_eq!(stats.by_type.get("Preference"), Some(&1));
        assert_eq!(stats.pinned, 1); // Only preference is pinned
    }

    #[tokio::test]
    async fn test_export_import() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Fact 1", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Fact 2", MemorySource::User)
            .await
            .unwrap();

        let json = manager.export().await.unwrap();
        manager.clear().await.unwrap();

        assert_eq!(manager.storage.count().await.unwrap(), 0);

        // Import requires deduplication to be off for exact count
        let config = MemoryConfig::default().without_deduplication();
        let new_manager = MemoryManager::new(config).await.unwrap();
        let count = new_manager.import(&json).await.unwrap();

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_relevant_context() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Rust uses Cargo for builds", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Python uses pip for packages", MemorySource::User)
            .await
            .unwrap();

        let context = manager.get_relevant_context("Rust", 5).await.unwrap();
        assert!(context.contains("Cargo"));
    }

    #[tokio::test]
    async fn test_deduplication() {
        // Use a low threshold to make deduplication work with our test strings
        let config = MemoryConfig {
            dedup_threshold: 0.5, // Lower threshold to catch similar strings
            ..MemoryConfig::default()
        };
        let manager = MemoryManager::new(config).await.unwrap();

        // Store similar memories
        manager
            .remember_fact("Rust uses Cargo", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Rust uses Cargo build", MemorySource::User)
            .await
            .unwrap();

        // With deduplication, count should be 1
        assert_eq!(manager.storage.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Test", MemorySource::User)
            .await
            .unwrap();
        manager.clear().await.unwrap();

        assert_eq!(manager.storage.count().await.unwrap(), 0);
    }

    #[test]
    fn test_config_builder() {
        let config = MemoryConfig::default()
            .max_memories(5000)
            .without_decay()
            .without_deduplication();

        assert_eq!(config.max_memories, 5000);
        assert!(!config.enable_decay);
        assert!(!config.deduplicate);
    }

    #[tokio::test]
    async fn test_shared_manager() {
        let config = MemoryConfig::default();
        let manager = create_memory_manager(config).await.unwrap();

        manager
            .remember_fact("Shared test", MemorySource::User)
            .await
            .unwrap();
        assert_eq!(manager.storage.count().await.unwrap(), 1);
    }

    #[test]
    fn test_calculate_similarity() {
        assert_eq!(calculate_similarity("hello world", "hello world"), 1.0);
        assert!(calculate_similarity("hello world", "goodbye moon") < 0.5);
        assert!(calculate_similarity("rust cargo", "rust cargo build") > 0.5);
    }
}
