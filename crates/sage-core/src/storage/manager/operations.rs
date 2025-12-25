//! Core database operations (query, execute, transaction)

use super::types::StorageStats;
use crate::storage::backend::{DatabaseBackend, DatabaseError, DatabaseValue, QueryResult};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Execute a query (SELECT)
pub async fn query(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    sql: &str,
    params: &[DatabaseValue],
) -> Result<QueryResult, DatabaseError> {
    let backend_lock = backend.read().await;
    let backend = backend_lock
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;

    let result = backend.query(sql, params).await;

    let mut stats = stats.write().await;
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
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    sql: &str,
    params: &[DatabaseValue],
) -> Result<QueryResult, DatabaseError> {
    let backend_lock = backend.read().await;
    let backend = backend_lock
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;

    let result = backend.execute(sql, params).await;

    let mut stats = stats.write().await;
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
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    statements: Vec<(&str, Vec<DatabaseValue>)>,
) -> Result<(), DatabaseError> {
    let backend_lock = backend.read().await;
    let backend = backend_lock
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;

    backend.transaction(statements).await
}
