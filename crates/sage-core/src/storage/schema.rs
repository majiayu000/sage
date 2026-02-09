//! Database schema and migration management

use super::backend::{DatabaseBackend, DatabaseError, DatabaseValue};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Schema version identifier
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion(pub u32);

impl SchemaVersion {
    /// Create a new version
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    /// Get version number
    pub fn version(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// A database migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Version this migration upgrades to
    pub version: SchemaVersion,
    /// Migration name/description
    pub name: String,
    /// SQL for upgrading (up migration)
    pub up_sql: String,
    /// SQL for downgrading (down migration)
    pub down_sql: Option<String>,
    /// Applied timestamp (set after migration runs)
    pub applied_at: Option<DateTime<Utc>>,
}

impl Migration {
    /// Create a new migration
    pub fn new(version: u32, name: impl Into<String>, up_sql: impl Into<String>) -> Self {
        Self {
            version: SchemaVersion::new(version),
            name: name.into(),
            up_sql: up_sql.into(),
            down_sql: None,
            applied_at: None,
        }
    }

    /// Add down migration SQL
    pub fn with_down(mut self, down_sql: impl Into<String>) -> Self {
        self.down_sql = Some(down_sql.into());
        self
    }
}

/// Migration runner for applying database migrations
pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Add a migration
    pub fn add_migration(&mut self, migration: Migration) {
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.version.0);
    }

    /// Add multiple migrations
    pub fn with_migrations(mut self, migrations: Vec<Migration>) -> Self {
        for m in migrations {
            self.add_migration(m);
        }
        self
    }

    /// Get current schema version from database
    pub async fn current_version(
        &self,
        backend: &dyn DatabaseBackend,
    ) -> Result<Option<SchemaVersion>, DatabaseError> {
        // Ensure migrations table exists
        self.ensure_migrations_table(backend).await?;

        // Query for latest version
        let result = backend
            .query(
                "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
                &[],
            )
            .await?;

        if let Some(row) = result.first() {
            if let Some(version) = row.get_i64("version") {
                let version = u32::try_from(version).map_err(|_| {
                    DatabaseError::Migration(format!(
                        "Invalid schema version in database: {} (expected non-negative u32)",
                        version
                    ))
                })?;
                return Ok(Some(SchemaVersion::new(version)));
            }
        }

        Ok(None)
    }

    /// Get pending migrations
    pub async fn pending_migrations(
        &self,
        backend: &dyn DatabaseBackend,
    ) -> Result<Vec<&Migration>, DatabaseError> {
        let current = self
            .current_version(backend)
            .await?
            .map(|v| v.0)
            .unwrap_or(0);

        Ok(self
            .migrations
            .iter()
            .filter(|m| m.version.0 > current)
            .collect())
    }

    /// Run all pending migrations
    pub async fn migrate(&self, backend: &dyn DatabaseBackend) -> Result<usize, DatabaseError> {
        self.ensure_migrations_table(backend).await?;

        let pending = self.pending_migrations(backend).await?;
        let count = pending.len();

        for migration in pending {
            tracing::info!(
                "Running migration {} ({}): {}",
                migration.version,
                migration.name,
                &migration.up_sql[..migration.up_sql.len().min(50)]
            );

            // Run the migration
            backend.execute(&migration.up_sql, &[]).await?;

            // Record the migration
            backend
                .execute(
                    "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?, ?, ?)",
                    &[
                        DatabaseValue::Int(migration.version.0 as i64),
                        DatabaseValue::Text(migration.name.clone()),
                        DatabaseValue::Text(Utc::now().to_rfc3339()),
                    ],
                )
                .await?;
        }

        if count > 0 {
            tracing::info!("Applied {} migrations", count);
        }

        Ok(count)
    }

    /// Rollback the last migration
    pub async fn rollback(&self, backend: &dyn DatabaseBackend) -> Result<bool, DatabaseError> {
        let current = self.current_version(backend).await?;

        if let Some(version) = current {
            // Find the migration
            if let Some(migration) = self.migrations.iter().find(|m| m.version == version) {
                if let Some(ref down_sql) = migration.down_sql {
                    tracing::info!("Rolling back migration {}: {}", version, migration.name);

                    // Run the down migration
                    backend.execute(down_sql, &[]).await?;

                    // Remove from migrations table
                    backend
                        .execute(
                            "DELETE FROM schema_migrations WHERE version = ?",
                            &[DatabaseValue::Int(version.0 as i64)],
                        )
                        .await?;

                    return Ok(true);
                } else {
                    return Err(DatabaseError::Migration(format!(
                        "Migration {} has no rollback SQL",
                        version
                    )));
                }
            }
        }

        Ok(false)
    }

    /// Get migration history
    pub async fn history(
        &self,
        backend: &dyn DatabaseBackend,
    ) -> Result<Vec<MigrationRecord>, DatabaseError> {
        self.ensure_migrations_table(backend).await?;

        let result = backend
            .query(
                "SELECT version, name, applied_at FROM schema_migrations ORDER BY version",
                &[],
            )
            .await?;

        let mut records = Vec::new();
        for row in result.rows {
            if let (Some(version), Some(name)) = (row.get_i64("version"), row.get_str("name")) {
                let version = u32::try_from(version).map_err(|_| {
                    DatabaseError::Migration(format!(
                        "Invalid migration history version in database: {} (expected non-negative u32)",
                        version
                    ))
                })?;

                records.push(MigrationRecord {
                    version: SchemaVersion::new(version),
                    name: name.to_string(),
                    applied_at: row.get_str("applied_at").map(|s| s.to_string()),
                });
            }
        }

        Ok(records)
    }

    async fn ensure_migrations_table(
        &self,
        backend: &dyn DatabaseBackend,
    ) -> Result<(), DatabaseError> {
        let sql = match backend.backend_type() {
            super::backend::BackendType::PostgreSQL => {
                "CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                )"
            }
            _ => {
                "CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                )"
            }
        };

        backend.execute(sql, &[]).await?;
        Ok(())
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// A record of an applied migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Version number
    pub version: SchemaVersion,
    /// Migration name
    pub name: String,
    /// When it was applied
    pub applied_at: Option<String>,
}

/// Built-in migrations for common tables
pub fn default_migrations() -> Vec<Migration> {
    vec![
        Migration::new(
            1,
            "create_kv_store",
            "CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .with_down("DROP TABLE IF EXISTS kv_store"),
        Migration::new(
            2,
            "create_sessions",
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT
            )",
        )
        .with_down("DROP TABLE IF EXISTS sessions"),
        Migration::new(
            3,
            "create_audit_log",
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                data TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .with_down("DROP TABLE IF EXISTS audit_log"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::backend::SqliteBackend;

    #[test]
    fn test_schema_version() {
        let v1 = SchemaVersion::new(1);
        let v2 = SchemaVersion::new(2);

        assert!(v1 < v2);
        assert_eq!(v1.to_string(), "v1");
    }

    #[test]
    fn test_migration_creation() {
        let migration = Migration::new(1, "test_migration", "CREATE TABLE test (id INTEGER)")
            .with_down("DROP TABLE test");

        assert_eq!(migration.version.0, 1);
        assert_eq!(migration.name, "test_migration");
        assert!(migration.down_sql.is_some());
    }

    #[test]
    fn test_migration_runner_ordering() {
        let mut runner = MigrationRunner::new();

        runner.add_migration(Migration::new(3, "third", "SQL3"));
        runner.add_migration(Migration::new(1, "first", "SQL1"));
        runner.add_migration(Migration::new(2, "second", "SQL2"));

        assert_eq!(runner.migrations[0].version.0, 1);
        assert_eq!(runner.migrations[1].version.0, 2);
        assert_eq!(runner.migrations[2].version.0, 3);
    }

    #[tokio::test]
    async fn test_migration_runner_migrate() {
        let backend = SqliteBackend::in_memory().await.unwrap();

        let runner = MigrationRunner::new().with_migrations(vec![
            Migration::new(
                1,
                "create_users",
                "CREATE TABLE users (id INTEGER, name TEXT)",
            ),
            Migration::new(
                2,
                "create_posts",
                "CREATE TABLE posts (id INTEGER, title TEXT)",
            ),
        ]);

        let count = runner.migrate(&backend).await.unwrap();
        assert_eq!(count, 2);

        // Running again should apply 0 migrations
        let count = runner.migrate(&backend).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_migration_history() {
        let backend = SqliteBackend::in_memory().await.unwrap();

        let runner = MigrationRunner::new().with_migrations(vec![Migration::new(
            1,
            "test_one",
            "CREATE TABLE test1 (id INTEGER)",
        )]);

        runner.migrate(&backend).await.unwrap();

        let history = runner.history(&backend).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].name, "test_one");
    }

    #[test]
    fn test_default_migrations() {
        let migrations = default_migrations();
        assert!(!migrations.is_empty());

        // All should have rollback SQL
        for m in &migrations {
            assert!(m.down_sql.is_some());
        }
    }
}
