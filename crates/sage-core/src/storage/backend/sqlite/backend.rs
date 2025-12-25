//! SQLite backend implementation
//!
//! Main backend struct and DatabaseBackend trait implementation.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::storage::backend::r#trait::DatabaseBackend;
use crate::storage::backend::types::{BackendType, DatabaseError, DatabaseValue, QueryResult};

use super::handlers::QueryHandler;

/// SQLite backend implementation
pub struct SqliteBackend {
    path: String,
    handler: QueryHandler,
    // Store table column definitions
    #[allow(dead_code)]
    schemas: Arc<RwLock<HashMap<String, Vec<String>>>>,
    connected: Arc<RwLock<bool>>,
}

impl SqliteBackend {
    /// Create new SQLite backend
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Create parent directory if needed
        if path_str != ":memory:" {
            if let Some(parent) = path.as_ref().parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        tracing::info!("SQLite backend connecting to: {}", path_str);

        let data = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            path: path_str,
            handler: QueryHandler {
                data: Arc::clone(&data),
            },
            schemas: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(true)),
        })
    }

    /// Create in-memory database
    pub async fn in_memory() -> Result<Self, DatabaseError> {
        Self::connect(":memory:").await
    }

    /// Get the database path
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[async_trait]
impl DatabaseBackend for SqliteBackend {
    fn backend_type(&self) -> BackendType {
        if self.path == ":memory:" {
            BackendType::InMemory
        } else {
            BackendType::SQLite
        }
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn ping(&self) -> Result<(), DatabaseError> {
        if *self.connected.read().await {
            Ok(())
        } else {
            Err(DatabaseError::Connection("Not connected".to_string()))
        }
    }

    async fn query(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        if !self.is_connected().await {
            return Err(DatabaseError::Connection("Not connected".to_string()));
        }

        self.handler.handle_query(sql, params).await
    }

    async fn execute(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        if !self.is_connected().await {
            return Err(DatabaseError::Connection("Not connected".to_string()));
        }

        let sql_lower = sql.to_lowercase();

        // Handle CREATE TABLE
        if sql_lower.starts_with("create table") {
            return self.handler.handle_create_table(sql).await;
        }

        // Handle INSERT
        if sql_lower.starts_with("insert") {
            return self.handler.handle_insert(sql, params).await;
        }

        // Handle DELETE
        if sql_lower.starts_with("delete") {
            return self.handler.handle_delete(sql, params).await;
        }

        Ok(QueryResult::from_affected(0))
    }

    async fn transaction(
        &self,
        statements: Vec<(&str, Vec<DatabaseValue>)>,
    ) -> Result<(), DatabaseError> {
        for (sql, params) in statements {
            self.execute(sql, &params).await?;
        }
        Ok(())
    }

    async fn version(&self) -> Result<String, DatabaseError> {
        Ok("SQLite 3.x (simulated)".to_string())
    }

    async fn close(&self) -> Result<(), DatabaseError> {
        *self.connected.write().await = false;
        Ok(())
    }
}
