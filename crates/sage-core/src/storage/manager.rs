//! Storage manager with automatic database fallback
//!
//! Provides a high-level interface that automatically handles database
//! connection failures and falls back to SQLite when PostgreSQL is unavailable.

use super::backend::{
    BackendType, DatabaseBackend, DatabaseError, DatabaseValue, PostgresBackend,
    QueryResult, SqliteBackend,
};
use super::config::{FallbackStrategy, StorageConfig};
use super::schema::{default_migrations, MigrationRunner};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connected to primary database
    Primary,
    /// Connected to fallback database
    Fallback,
    /// Not connected
    #[default]
    Disconnected,
    /// Reconnecting
    Reconnecting,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary (PostgreSQL)"),
            Self::Fallback => write!(f, "Fallback (SQLite)"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Reconnecting => write!(f, "Reconnecting..."),
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total queries executed
    pub total_queries: u64,
    /// Successful queries
    pub successful_queries: u64,
    /// Failed queries
    pub failed_queries: u64,
    /// Times fallback was triggered
    pub fallback_count: u64,
    /// Times reconnected to primary
    pub reconnect_count: u64,
    /// Current backend type
    pub backend_type: Option<BackendType>,
    /// Connection status
    pub status: ConnectionStatus,
    /// Last error message
    pub last_error: Option<String>,
    /// Connected since
    pub connected_since: Option<DateTime<Utc>>,
}

/// Storage manager with automatic fallback
pub struct StorageManager {
    config: StorageConfig,
    backend: Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: Arc<RwLock<StorageStats>>,
    migration_runner: MigrationRunner,
}

impl StorageManager {
    /// Connect to storage with automatic fallback
    pub async fn connect(config: StorageConfig) -> Result<Self, DatabaseError> {
        let manager = Self {
            config: config.clone(),
            backend: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            migration_runner: MigrationRunner::new().with_migrations(default_migrations()),
        };

        manager.establish_connection().await?;

        // Run migrations if enabled
        if config.auto_migrate {
            manager.run_migrations().await?;
        }

        Ok(manager)
    }

    /// Connect with custom migrations
    pub async fn connect_with_migrations(
        config: StorageConfig,
        migrations: Vec<super::schema::Migration>,
    ) -> Result<Self, DatabaseError> {
        let manager = Self {
            config: config.clone(),
            backend: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            migration_runner: MigrationRunner::new().with_migrations(migrations),
        };

        manager.establish_connection().await?;

        if config.auto_migrate {
            manager.run_migrations().await?;
        }

        Ok(manager)
    }

    /// Establish database connection with fallback logic
    async fn establish_connection(&self) -> Result<(), DatabaseError> {
        match self.config.fallback_strategy {
            FallbackStrategy::SqliteOnly => {
                self.connect_sqlite().await
            }
            FallbackStrategy::FailFast => {
                if self.config.should_try_postgres() {
                    self.connect_postgres().await
                } else {
                    self.connect_sqlite().await
                }
            }
            FallbackStrategy::AutoFallback => {
                if self.config.should_try_postgres() {
                    match self.connect_postgres().await {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            tracing::warn!(
                                "PostgreSQL connection failed: {}. Falling back to SQLite.",
                                e
                            );
                            self.stats.write().await.fallback_count += 1;
                            self.connect_sqlite().await
                        }
                    }
                } else {
                    self.connect_sqlite().await
                }
            }
            FallbackStrategy::RetryThenFallback => {
                if self.config.should_try_postgres() {
                    match self.connect_postgres_with_retry().await {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            tracing::warn!(
                                "PostgreSQL connection failed after retries: {}. Falling back to SQLite.",
                                e
                            );
                            self.stats.write().await.fallback_count += 1;
                            self.connect_sqlite().await
                        }
                    }
                } else {
                    self.connect_sqlite().await
                }
            }
        }
    }

    /// Connect to PostgreSQL
    async fn connect_postgres(&self) -> Result<(), DatabaseError> {
        let pg_config = self.config.postgres.as_ref().ok_or_else(|| {
            DatabaseError::Connection("PostgreSQL not configured".to_string())
        })?;

        tracing::info!("Attempting PostgreSQL connection...");

        let backend = PostgresBackend::connect(&pg_config.connection_string).await?;

        *self.backend.write().await = Some(Box::new(backend));

        let mut stats = self.stats.write().await;
        stats.backend_type = Some(BackendType::PostgreSQL);
        stats.status = ConnectionStatus::Primary;
        stats.connected_since = Some(Utc::now());

        tracing::info!("Connected to PostgreSQL successfully");
        Ok(())
    }

    /// Connect to PostgreSQL with retry
    async fn connect_postgres_with_retry(&self) -> Result<(), DatabaseError> {
        let mut delay = self.config.retry.initial_delay;
        let mut last_error = None;

        for attempt in 1..=self.config.retry.max_retries {
            tracing::info!("PostgreSQL connection attempt {}/{}", attempt, self.config.retry.max_retries);

            match self.connect_postgres().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.config.retry.max_retries {
                        tracing::warn!("Connection failed, retrying in {:?}...", delay);
                        sleep(delay).await;

                        // Exponential backoff
                        delay = Duration::from_secs_f32(
                            delay.as_secs_f32() * self.config.retry.backoff_multiplier,
                        )
                        .min(self.config.retry.max_delay);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            DatabaseError::Connection("Max retries exceeded".to_string())
        }))
    }

    /// Connect to SQLite
    async fn connect_sqlite(&self) -> Result<(), DatabaseError> {
        tracing::info!("Connecting to SQLite at: {:?}", self.config.sqlite.path);

        let backend = SqliteBackend::connect(&self.config.sqlite.path).await?;
        let backend_type = backend.backend_type();

        *self.backend.write().await = Some(Box::new(backend));

        let mut stats = self.stats.write().await;
        stats.backend_type = Some(backend_type);
        stats.status = ConnectionStatus::Fallback;
        stats.connected_since = Some(Utc::now());

        tracing::info!("Connected to SQLite successfully");
        Ok(())
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<usize, DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;

        self.migration_runner.migrate(backend.as_ref()).await
    }

    /// Execute a query (SELECT)
    pub async fn query(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;

        let result = backend.query(sql, params).await;

        let mut stats = self.stats.write().await;
        stats.total_queries += 1;
        match &result {
            Ok(_) => stats.successful_queries += 1,
            Err(e) => {
                stats.failed_queries += 1;
                stats.last_error = Some(e.to_string());
            }
        }

        result
    }

    /// Execute a statement (INSERT/UPDATE/DELETE)
    pub async fn execute(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;

        let result = backend.execute(sql, params).await;

        let mut stats = self.stats.write().await;
        stats.total_queries += 1;
        match &result {
            Ok(_) => stats.successful_queries += 1,
            Err(e) => {
                stats.failed_queries += 1;
                stats.last_error = Some(e.to_string());
            }
        }

        result
    }

    /// Execute multiple statements in a transaction
    pub async fn transaction(
        &self,
        statements: Vec<(&str, Vec<DatabaseValue>)>,
    ) -> Result<(), DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;

        backend.transaction(statements).await
    }

    /// Get a value from key-value store
    pub async fn get(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let result = self
            .query(
                "SELECT value FROM kv_store WHERE key = ?",
                &[DatabaseValue::Text(key.to_string())],
            )
            .await?;

        Ok(result.first().and_then(|row| row.get_str("value").map(|s| s.to_string())))
    }

    /// Set a value in key-value store
    pub async fn set(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();

        // Try INSERT OR REPLACE for SQLite, or upsert for PostgreSQL
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;

        let sql = match backend.backend_type() {
            BackendType::PostgreSQL => {
                "INSERT INTO kv_store (key, value, created_at, updated_at) \
                 VALUES (?, ?, ?, ?) \
                 ON CONFLICT (key) DO UPDATE SET value = ?, updated_at = ?"
            }
            _ => {
                "INSERT OR REPLACE INTO kv_store (key, value, created_at, updated_at) \
                 VALUES (?, ?, ?, ?)"
            }
        };

        drop(backend); // Release read lock before execute

        self.execute(
            sql,
            &[
                DatabaseValue::Text(key.to_string()),
                DatabaseValue::Text(value.to_string()),
                DatabaseValue::Text(now.clone()),
                DatabaseValue::Text(now),
            ],
        )
        .await?;

        Ok(())
    }

    /// Delete a value from key-value store
    pub async fn delete(&self, key: &str) -> Result<bool, DatabaseError> {
        let result = self
            .execute(
                "DELETE FROM kv_store WHERE key = ?",
                &[DatabaseValue::Text(key.to_string())],
            )
            .await?;

        Ok(result.rows_affected > 0)
    }

    /// Get JSON value from key-value store
    pub async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, DatabaseError> {
        if let Some(value) = self.get(key).await? {
            let parsed: T = serde_json::from_str(&value)
                .map_err(|e| DatabaseError::Serialization(e.to_string()))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Set JSON value in key-value store
    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(value)
            .map_err(|e| DatabaseError::Serialization(e.to_string()))?;
        self.set(key, &json).await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        let backend = self.backend.read().await;
        if let Some(ref backend) = *backend {
            backend.is_connected().await
        } else {
            false
        }
    }

    /// Ping the database
    pub async fn ping(&self) -> Result<(), DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;
        backend.ping().await
    }

    /// Get current backend type
    pub async fn backend_type(&self) -> Option<BackendType> {
        let backend = self.backend.read().await;
        backend.as_ref().map(|b| b.backend_type())
    }

    /// Get connection status
    pub async fn status(&self) -> ConnectionStatus {
        self.stats.read().await.status
    }

    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        self.stats.read().await.clone()
    }

    /// Get database version
    pub async fn version(&self) -> Result<String, DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend.as_ref().ok_or_else(|| {
            DatabaseError::Connection("Not connected".to_string())
        })?;
        backend.version().await
    }

    /// Close the connection
    pub async fn close(&self) -> Result<(), DatabaseError> {
        let backend = self.backend.write().await;
        if let Some(ref backend) = *backend {
            backend.close().await?;
        }
        self.stats.write().await.status = ConnectionStatus::Disconnected;
        Ok(())
    }

    /// Get health check info
    pub async fn health(&self) -> HealthInfo {
        let stats = self.stats.read().await.clone();
        let connected = self.is_connected().await;
        let backend_type = self.backend_type().await;

        HealthInfo {
            connected,
            backend_type,
            status: stats.status,
            uptime: stats.connected_since.map(|t| Utc::now() - t),
            total_queries: stats.total_queries,
            error_rate: if stats.total_queries > 0 {
                stats.failed_queries as f64 / stats.total_queries as f64
            } else {
                0.0
            },
            fallback_count: stats.fallback_count,
        }
    }
}

/// Health check information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthInfo {
    /// Is connected
    pub connected: bool,
    /// Current backend type
    pub backend_type: Option<BackendType>,
    /// Connection status
    pub status: ConnectionStatus,
    /// Uptime duration
    pub uptime: Option<chrono::Duration>,
    /// Total queries executed
    pub total_queries: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Number of times fallback was triggered
    pub fallback_count: u64,
}

/// Thread-safe shared storage manager
pub type SharedStorageManager = Arc<StorageManager>;

/// Create a shared storage manager
pub async fn create_storage_manager(
    config: StorageConfig,
) -> Result<SharedStorageManager, DatabaseError> {
    Ok(Arc::new(StorageManager::connect(config).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(ConnectionStatus::Primary.to_string(), "Primary (PostgreSQL)");
        assert_eq!(ConnectionStatus::Fallback.to_string(), "Fallback (SQLite)");
    }

    #[tokio::test]
    async fn test_fail_fast_strategy() {
        // With FailFast and no PostgreSQL configured, should connect to SQLite
        let config = StorageConfig::default()
            .with_fallback_strategy(FallbackStrategy::FailFast);

        let manager = StorageManager::connect(config).await.unwrap();
        assert!(manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_shared_storage_manager() {
        let config = StorageConfig::in_memory();
        let manager = create_storage_manager(config).await.unwrap();

        // Clone and use from multiple "threads"
        let m1 = manager.clone();
        let m2 = manager.clone();

        m1.set("shared_key", "value1").await.unwrap();
        let value = m2.get("shared_key").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }
}
