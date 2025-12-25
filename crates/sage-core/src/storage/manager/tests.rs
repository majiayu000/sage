//! Storage manager tests

#[cfg(test)]
mod tests {
    use crate::storage::config::{FallbackStrategy, StorageConfig};
    use crate::storage::manager::{ConnectionStatus, StorageManager};
    use crate::storage::backend::BackendType;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn test_sqlite_only_connection() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        assert!(manager.is_connected().await);
        assert_eq!(manager.backend_type().await, Some(BackendType::InMemory));
        assert_eq!(manager.status().await, ConnectionStatus::Fallback);
    }

    #[tokio::test]
    async fn test_auto_fallback_from_postgres() {
        // Configure with PostgreSQL that will fail, and SQLite fallback
        let config = StorageConfig::default()
            .with_primary("postgresql://localhost/nonexistent")
            .with_fallback_sqlite(":memory:")
            .with_fallback_strategy(FallbackStrategy::AutoFallback);

        let manager = StorageManager::connect(config).await.unwrap();

        // Should have fallen back to SQLite
        assert!(manager.is_connected().await);
        assert_eq!(manager.backend_type().await, Some(BackendType::InMemory));
        assert_eq!(manager.status().await, ConnectionStatus::Fallback);

        // Check stats
        let stats = manager.stats().await;
        assert_eq!(stats.fallback_count, 1);
    }

    #[tokio::test]
    async fn test_key_value_operations() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        // Set and get
        manager.set("test_key", "test_value").await.unwrap();
        let value = manager.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Get non-existent
        let missing = manager.get("nonexistent").await.unwrap();
        assert!(missing.is_none());

        // Delete
        let deleted = manager.delete("test_key").await.unwrap();
        assert!(deleted);

        // Verify deleted
        let value = manager.get("test_key").await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_json_operations() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        manager.set_json("json_key", &data).await.unwrap();
        let retrieved: Option<TestData> = manager.get_json("json_key").await.unwrap();

        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_query_stats() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        // Execute some queries
        manager.set("k1", "v1").await.unwrap();
        manager.set("k2", "v2").await.unwrap();
        manager.get("k1").await.unwrap();

        let stats = manager.stats().await;
        assert!(stats.total_queries >= 3);
        assert!(stats.successful_queries >= 3);
    }

    #[tokio::test]
    async fn test_ping() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        assert!(manager.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        let health = manager.health().await;
        assert!(health.connected);
        assert_eq!(health.status, ConnectionStatus::Fallback);
        assert_eq!(health.error_rate, 0.0);
    }

    #[tokio::test]
    async fn test_close() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        assert!(manager.is_connected().await);
        manager.close().await.unwrap();
        assert_eq!(manager.status().await, ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_migrations_run() {
        let config = StorageConfig::in_memory();
        let manager = StorageManager::connect(config).await.unwrap();

        // Migrations should have been run (kv_store table exists)
        let result = manager.set("migration_test", "value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_status_display() {
        assert_eq!(
            ConnectionStatus::Primary.to_string(),
            "Primary (PostgreSQL)"
        );
        assert_eq!(ConnectionStatus::Fallback.to_string(), "Fallback (SQLite)");
    }

    #[tokio::test]
    async fn test_fail_fast_strategy() {
        // With FailFast and no PostgreSQL configured, should connect to SQLite
        let config = StorageConfig::default().with_fallback_strategy(FallbackStrategy::FailFast);

        let manager = StorageManager::connect(config).await.unwrap();
        assert!(manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_shared_storage_manager() {
        let config = StorageConfig::in_memory();
        let manager = super::super::create_storage_manager(config).await.unwrap();

        // Clone and use from multiple "threads"
        let m1 = manager.clone();
        let m2 = manager.clone();

        m1.set("shared_key", "value1").await.unwrap();
        let value = m2.get("shared_key").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }
}
