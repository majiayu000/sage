//! Storage manager with automatic database fallback
//!
//! Provides a high-level interface that automatically handles database
//! connection failures and falls back to SQLite when PostgreSQL is unavailable.

mod connection;
mod kv_store;
mod maintenance;
mod operations;
mod types;

#[cfg(test)]
mod tests;

use crate::storage::backend::{BackendType, DatabaseBackend, DatabaseError, DatabaseValue, QueryResult};
use crate::storage::config::StorageConfig;
use crate::storage::schema::{MigrationRunner, default_migrations};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export types
pub use types::{ConnectionStatus, HealthInfo, StorageStats};

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

        connection::establish_connection(&manager.config, &manager.backend, &manager.stats).await?;

        // Run migrations if enabled
        if config.auto_migrate {
            manager.run_migrations().await?;
        }

        Ok(manager)
    }

    /// Connect with custom migrations
    pub async fn connect_with_migrations(
        config: StorageConfig,
        migrations: Vec<crate::storage::schema::Migration>,
    ) -> Result<Self, DatabaseError> {
        let manager = Self {
            config: config.clone(),
            backend: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            migration_runner: MigrationRunner::new().with_migrations(migrations),
        };

        connection::establish_connection(&manager.config, &manager.backend, &manager.stats).await?;

        if config.auto_migrate {
            manager.run_migrations().await?;
        }

        Ok(manager)
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<usize, DatabaseError> {
        let backend = self.backend.read().await;
        let backend = backend
            .as_ref()
            .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;

        self.migration_runner.migrate(backend.as_ref()).await
    }

    /// Execute a query (SELECT)
    pub async fn query(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        operations::query(&self.backend, &self.stats, sql, params).await
    }

    /// Execute a statement (INSERT/UPDATE/DELETE)
    pub async fn execute(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        operations::execute(&self.backend, &self.stats, sql, params).await
    }

    /// Execute multiple statements in a transaction
    pub async fn transaction(
        &self,
        statements: Vec<(&str, Vec<DatabaseValue>)>,
    ) -> Result<(), DatabaseError> {
        operations::transaction(&self.backend, statements).await
    }

    /// Get a value from key-value store
    pub async fn get(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        kv_store::get(&self.backend, &self.stats, key).await
    }

    /// Set a value in key-value store
    pub async fn set(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        kv_store::set(&self.backend, &self.stats, key, value).await
    }

    /// Delete a value from key-value store
    pub async fn delete(&self, key: &str) -> Result<bool, DatabaseError> {
        kv_store::delete(&self.backend, &self.stats, key).await
    }

    /// Get JSON value from key-value store
    pub async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, DatabaseError> {
        kv_store::get_json(&self.backend, &self.stats, key).await
    }

    /// Set JSON value in key-value store
    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DatabaseError> {
        kv_store::set_json(&self.backend, &self.stats, key, value).await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        maintenance::is_connected(&self.backend).await
    }

    /// Ping the database
    pub async fn ping(&self) -> Result<(), DatabaseError> {
        maintenance::ping(&self.backend).await
    }

    /// Get current backend type
    pub async fn backend_type(&self) -> Option<BackendType> {
        maintenance::backend_type(&self.backend).await
    }

    /// Get connection status
    pub async fn status(&self) -> ConnectionStatus {
        maintenance::status(&self.stats).await
    }

    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        maintenance::stats(&self.stats).await
    }

    /// Get database version
    pub async fn version(&self) -> Result<String, DatabaseError> {
        maintenance::version(&self.backend).await
    }

    /// Close the connection
    pub async fn close(&self) -> Result<(), DatabaseError> {
        maintenance::close(&self.backend, &self.stats).await
    }

    /// Get health check info
    pub async fn health(&self) -> HealthInfo {
        maintenance::health(&self.backend, &self.stats).await
    }
}

/// Thread-safe shared storage manager
pub type SharedStorageManager = Arc<StorageManager>;

/// Create a shared storage manager
pub async fn create_storage_manager(
    config: StorageConfig,
) -> Result<SharedStorageManager, DatabaseError> {
    Ok(Arc::new(StorageManager::connect(config).await?))
}
