//! Storage tests

#[cfg(test)]
mod tests {
    use crate::memory::storage::{FileMemoryStorage, InMemoryStorage, MemoryStorage};
    use crate::memory::types::{Memory, MemoryCategory, MemoryQuery, MemoryType};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_in_memory_store() {
        let storage = InMemoryStorage::new();

        let memory = Memory::fact("Test fact");
        let id = storage.store(memory.clone()).await.unwrap();

        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test fact");
    }

    #[tokio::test]
    async fn test_in_memory_update() {
        let storage = InMemoryStorage::new();

        let memory = Memory::fact("Original");
        let id = storage.store(memory).await.unwrap();

        let mut updated = storage.get(&id).await.unwrap().unwrap();
        updated.content = "Updated".to_string();
        storage.update(updated).await.unwrap();

        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated");
    }

    #[tokio::test]
    async fn test_in_memory_delete() {
        let storage = InMemoryStorage::new();

        let memory = Memory::fact("To delete");
        let id = storage.store(memory).await.unwrap();

        storage.delete(&id).await.unwrap();
        assert!(storage.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_search_by_text() {
        let storage = InMemoryStorage::new();

        storage
            .store(Memory::fact("Rust is a systems language"))
            .await
            .unwrap();
        storage
            .store(Memory::fact("Python is interpreted"))
            .await
            .unwrap();

        let query = MemoryQuery::new().text("Rust");
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].memory.content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_in_memory_search_by_type() {
        let storage = InMemoryStorage::new();

        storage.store(Memory::fact("A fact")).await.unwrap();
        storage
            .store(Memory::preference("A preference"))
            .await
            .unwrap();

        let query = MemoryQuery::new().memory_type(MemoryType::Fact);
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.memory_type, MemoryType::Fact);
    }

    #[tokio::test]
    async fn test_in_memory_search_by_category() {
        let storage = InMemoryStorage::new();

        storage
            .store(Memory::fact("Project fact").with_category(MemoryCategory::Project))
            .await
            .unwrap();
        storage
            .store(Memory::fact("Global fact").with_category(MemoryCategory::Global))
            .await
            .unwrap();

        let query = MemoryQuery::new().category(MemoryCategory::Project);
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.category, MemoryCategory::Project);
    }

    #[tokio::test]
    async fn test_in_memory_search_by_tag() {
        let storage = InMemoryStorage::new();

        let mut m1 = Memory::fact("Tagged");
        m1.add_tag("important");
        storage.store(m1).await.unwrap();

        storage.store(Memory::fact("Not tagged")).await.unwrap();

        let query = MemoryQuery::new().tag("important");
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].memory.has_tag("important"));
    }

    #[tokio::test]
    async fn test_in_memory_search_with_limit() {
        let storage = InMemoryStorage::new();

        for i in 0..10 {
            storage
                .store(Memory::fact(format!("Fact {}", i)))
                .await
                .unwrap();
        }

        let query = MemoryQuery::new().limit(5);
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_in_memory_list() {
        let storage = InMemoryStorage::new();

        for i in 0..5 {
            storage
                .store(Memory::fact(format!("Fact {}", i)))
                .await
                .unwrap();
        }

        let all = storage.list(0, 100).await.unwrap();
        assert_eq!(all.len(), 5);

        let partial = storage.list(2, 2).await.unwrap();
        assert_eq!(partial.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_count() {
        let storage = InMemoryStorage::new();

        for i in 0..3 {
            storage
                .store(Memory::fact(format!("Fact {}", i)))
                .await
                .unwrap();
        }

        assert_eq!(storage.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_in_memory_clear() {
        let storage = InMemoryStorage::new();

        storage.store(Memory::fact("Test")).await.unwrap();
        storage.clear().await.unwrap();

        assert_eq!(storage.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_file_storage_basic() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("memories.json");

        let storage = FileMemoryStorage::new(&path).await.unwrap();

        let memory = Memory::fact("Test fact");
        let id = storage.store(memory).await.unwrap();

        // Verify file was created
        assert!(path.exists());

        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test fact");
    }

    #[tokio::test]
    async fn test_file_storage_persistence() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("memories.json");

        // Store memory
        {
            let storage = FileMemoryStorage::new(&path).await.unwrap();
            storage
                .store(Memory::fact("Persistent fact"))
                .await
                .unwrap();
        }

        // Load in new instance
        {
            let storage = FileMemoryStorage::new(&path).await.unwrap();
            let all = storage.list(0, 100).await.unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].content, "Persistent fact");
        }
    }

    #[tokio::test]
    async fn test_file_storage_max_memories() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("memories.json");

        let storage = FileMemoryStorage::new(&path)
            .await
            .unwrap()
            .with_max_memories(2);

        storage.store(Memory::fact("First")).await.unwrap();
        storage.store(Memory::fact("Second")).await.unwrap();

        let result = storage.store(Memory::fact("Third")).await;
        assert!(result.is_err());
    }
}
