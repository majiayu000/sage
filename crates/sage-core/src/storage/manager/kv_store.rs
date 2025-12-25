//! Key-value store operations

use super::operations::{execute, query};
use super::types::StorageStats;
use crate::storage::backend::{BackendType, DatabaseBackend, DatabaseError, DatabaseValue};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Get a value from key-value store
pub async fn get(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    key: &str,
) -> Result<Option<String>, DatabaseError> {
    let result = query(
        backend,
        stats,
        "SELECT value FROM kv_store WHERE key = ?",
        &[DatabaseValue::Text(key.to_string())],
    )
    .await?;

    Ok(result
        .first()
        .and_then(|row| row.get_str("value").map(|s| s.to_string())))
}

/// Set a value in key-value store
pub async fn set(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    key: &str,
    value: &str,
) -> Result<(), DatabaseError> {
    let now = Utc::now().to_rfc3339();

    // Try INSERT OR REPLACE for SQLite, or upsert for PostgreSQL
    let backend_lock = backend.read().await;
    let backend_ref = backend_lock
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;

    let sql = match backend_ref.backend_type() {
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

    let _ = backend_ref; // Release read lock before execute

    execute(
        backend,
        stats,
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
pub async fn delete(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    key: &str,
) -> Result<bool, DatabaseError> {
    let result = execute(
        backend,
        stats,
        "DELETE FROM kv_store WHERE key = ?",
        &[DatabaseValue::Text(key.to_string())],
    )
    .await?;

    Ok(result.rows_affected > 0)
}

/// Get JSON value from key-value store
pub async fn get_json<T: for<'de> Deserialize<'de>>(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    key: &str,
) -> Result<Option<T>, DatabaseError> {
    if let Some(value) = get(backend, stats, key).await? {
        let parsed: T = serde_json::from_str(&value)
            .map_err(|e| DatabaseError::Serialization(e.to_string()))?;
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

/// Set JSON value in key-value store
pub async fn set_json<T: Serialize>(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
    key: &str,
    value: &T,
) -> Result<(), DatabaseError> {
    let json =
        serde_json::to_string(value).map_err(|e| DatabaseError::Serialization(e.to_string()))?;
    set(backend, stats, key, &json).await
}
