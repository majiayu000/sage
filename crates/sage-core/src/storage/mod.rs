//! Database storage with automatic fallback
//!
//! Provides a unified storage interface that supports multiple database backends
//! with automatic fallback from PostgreSQL to SQLite when connection fails.
//!
//! # Features
//! - PostgreSQL as primary database (production)
//! - SQLite as fallback (development/local)
//! - Automatic failover with configurable retry
//! - Connection health monitoring
//! - Schema migration support
//!
//! # Example
//! ```ignore
//! let config = StorageConfig::default()
//!     .with_primary("postgresql://localhost/mydb")
//!     .with_fallback_sqlite("data/local.db");
//!
//! let storage = StorageManager::connect(config).await?;
//! // Will try PostgreSQL first, fallback to SQLite if connection fails
//! ```

pub mod backend;
pub mod config;
pub mod manager;
pub mod schema;

pub use backend::{
    BackendType, DatabaseBackend, DatabaseError, DatabaseRow, DatabaseValue, PostgresBackend,
    QueryResult, SqliteBackend,
};
pub use config::{ConnectionPool, FallbackStrategy, PostgresConfig, SqliteConfig, StorageConfig};
pub use manager::{
    ConnectionStatus, HealthInfo, SharedStorageManager, StorageManager, StorageStats,
    create_storage_manager,
};
pub use schema::{Migration, MigrationRunner, SchemaVersion};
