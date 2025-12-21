//! Storage configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Fallback strategy when primary database fails
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackStrategy {
    /// Fail immediately if primary is unavailable
    FailFast,
    /// Automatically fallback to SQLite
    AutoFallback,
    /// Try primary with retries, then fallback
    RetryThenFallback,
    /// Use SQLite only (no primary)
    SqliteOnly,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self::AutoFallback
    }
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPool {
    /// Minimum connections to keep
    pub min_connections: u32,
    /// Maximum connections allowed
    pub max_connections: u32,
    /// Connection timeout
    #[serde(with = "humantime_serde")]
    pub connect_timeout: Duration,
    /// Idle timeout before closing connection
    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    #[serde(with = "humantime_serde")]
    pub max_lifetime: Duration,
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connect_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Initial delay between retries
    #[serde(with = "humantime_serde")]
    pub initial_delay: Duration,
    /// Maximum delay between retries
    #[serde(with = "humantime_serde")]
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

/// PostgreSQL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Connection string (e.g., "postgresql://user:pass@host/db")
    pub connection_string: String,
    /// Connection pool settings
    pub pool: ConnectionPool,
    /// SSL mode
    pub ssl_mode: SslMode,
    /// Schema name (default: public)
    pub schema: String,
}

impl PostgresConfig {
    /// Create from connection string
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            pool: ConnectionPool::default(),
            ssl_mode: SslMode::Prefer,
            schema: "public".to_string(),
        }
    }

    /// Set SSL mode
    pub fn with_ssl(mut self, mode: SslMode) -> Self {
        self.ssl_mode = mode;
        self
    }

    /// Set schema
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = schema.into();
        self
    }

    /// Set pool configuration
    pub fn with_pool(mut self, pool: ConnectionPool) -> Self {
        self.pool = pool;
        self
    }
}

/// SSL mode for PostgreSQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
}

impl Default for SslMode {
    fn default() -> Self {
        Self::Prefer
    }
}

/// SQLite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Database file path
    pub path: PathBuf,
    /// Create if not exists
    pub create_if_missing: bool,
    /// Enable WAL mode for better concurrency
    pub wal_mode: bool,
    /// Busy timeout
    #[serde(with = "humantime_serde")]
    pub busy_timeout: Duration,
    /// Cache size in KB (negative = number of pages)
    pub cache_size_kb: i32,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/storage.db"),
            create_if_missing: true,
            wal_mode: true,
            busy_timeout: Duration::from_secs(5),
            cache_size_kb: 2048,
        }
    }
}

impl SqliteConfig {
    /// Create with specific path
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    /// In-memory database (for testing)
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::from(":memory:"),
            create_if_missing: true,
            wal_mode: false, // WAL not supported for in-memory
            busy_timeout: Duration::from_secs(5),
            cache_size_kb: 2048,
        }
    }

    /// Set WAL mode
    pub fn with_wal(mut self, enabled: bool) -> Self {
        self.wal_mode = enabled;
        self
    }

    /// Set cache size
    pub fn with_cache_size(mut self, kb: i32) -> Self {
        self.cache_size_kb = kb;
        self
    }
}

/// Main storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// PostgreSQL configuration (primary)
    pub postgres: Option<PostgresConfig>,
    /// SQLite configuration (fallback)
    pub sqlite: SqliteConfig,
    /// Fallback strategy
    pub fallback_strategy: FallbackStrategy,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Enable connection health checks
    pub health_check_enabled: bool,
    /// Health check interval
    #[serde(with = "humantime_serde")]
    pub health_check_interval: Duration,
    /// Auto-migrate on startup
    pub auto_migrate: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            postgres: None,
            sqlite: SqliteConfig::default(),
            fallback_strategy: FallbackStrategy::AutoFallback,
            retry: RetryConfig::default(),
            health_check_enabled: true,
            health_check_interval: Duration::from_secs(30),
            auto_migrate: true,
        }
    }
}

impl StorageConfig {
    /// Create with PostgreSQL as primary
    pub fn with_postgres(connection_string: impl Into<String>) -> Self {
        Self {
            postgres: Some(PostgresConfig::new(connection_string)),
            ..Default::default()
        }
    }

    /// Create SQLite-only config
    pub fn sqlite_only(path: impl Into<PathBuf>) -> Self {
        Self {
            postgres: None,
            sqlite: SqliteConfig::new(path),
            fallback_strategy: FallbackStrategy::SqliteOnly,
            ..Default::default()
        }
    }

    /// Create in-memory config (for testing)
    pub fn in_memory() -> Self {
        Self {
            postgres: None,
            sqlite: SqliteConfig::in_memory(),
            fallback_strategy: FallbackStrategy::SqliteOnly,
            auto_migrate: true,
            ..Default::default()
        }
    }

    /// Set primary PostgreSQL
    pub fn with_primary(mut self, connection_string: impl Into<String>) -> Self {
        self.postgres = Some(PostgresConfig::new(connection_string));
        self
    }

    /// Set fallback SQLite path
    pub fn with_fallback_sqlite(mut self, path: impl Into<PathBuf>) -> Self {
        self.sqlite = SqliteConfig::new(path);
        self
    }

    /// Set fallback strategy
    pub fn with_fallback_strategy(mut self, strategy: FallbackStrategy) -> Self {
        self.fallback_strategy = strategy;
        self
    }

    /// Disable auto-migration
    pub fn without_auto_migrate(mut self) -> Self {
        self.auto_migrate = false;
        self
    }

    /// Disable health checks
    pub fn without_health_check(mut self) -> Self {
        self.health_check_enabled = false;
        self
    }

    /// Check if PostgreSQL is configured
    pub fn has_postgres(&self) -> bool {
        self.postgres.is_some()
    }

    /// Check if should try PostgreSQL first
    pub fn should_try_postgres(&self) -> bool {
        self.has_postgres() && self.fallback_strategy != FallbackStrategy::SqliteOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StorageConfig::default();
        assert!(config.postgres.is_none());
        assert_eq!(config.fallback_strategy, FallbackStrategy::AutoFallback);
        assert!(config.auto_migrate);
    }

    #[test]
    fn test_postgres_config() {
        let config = StorageConfig::with_postgres("postgresql://localhost/test");
        assert!(config.postgres.is_some());
        assert!(config.should_try_postgres());
    }

    #[test]
    fn test_sqlite_only() {
        let config = StorageConfig::sqlite_only("test.db");
        assert!(config.postgres.is_none());
        assert!(!config.should_try_postgres());
        assert_eq!(config.fallback_strategy, FallbackStrategy::SqliteOnly);
    }

    #[test]
    fn test_in_memory() {
        let config = StorageConfig::in_memory();
        assert_eq!(config.sqlite.path.to_string_lossy(), ":memory:");
    }

    #[test]
    fn test_connection_pool_defaults() {
        let pool = ConnectionPool::default();
        assert_eq!(pool.min_connections, 1);
        assert_eq!(pool.max_connections, 10);
    }

    #[test]
    fn test_retry_config() {
        let retry = RetryConfig::default();
        assert_eq!(retry.max_retries, 3);
        assert_eq!(retry.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_postgres_config_builder() {
        let config = PostgresConfig::new("postgresql://localhost/test")
            .with_ssl(SslMode::Require)
            .with_schema("myschema");

        assert_eq!(config.ssl_mode, SslMode::Require);
        assert_eq!(config.schema, "myschema");
    }

    #[test]
    fn test_sqlite_config_builder() {
        let config = SqliteConfig::new("data/app.db")
            .with_wal(false)
            .with_cache_size(4096);

        assert!(!config.wal_mode);
        assert_eq!(config.cache_size_kb, 4096);
    }

    #[test]
    fn test_storage_config_builder() {
        let config = StorageConfig::default()
            .with_primary("postgresql://localhost/prod")
            .with_fallback_sqlite("data/fallback.db")
            .with_fallback_strategy(FallbackStrategy::RetryThenFallback)
            .without_auto_migrate();

        assert!(config.has_postgres());
        assert_eq!(
            config.fallback_strategy,
            FallbackStrategy::RetryThenFallback
        );
        assert!(!config.auto_migrate);
    }
}
