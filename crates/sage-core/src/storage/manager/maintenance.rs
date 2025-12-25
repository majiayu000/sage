//! Database maintenance operations (ping, health, stats, version, close)

use super::types::{ConnectionStatus, HealthInfo, StorageStats};
use crate::storage::backend::{BackendType, DatabaseBackend, DatabaseError};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Check if connected
pub async fn is_connected(backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>) -> bool {
    let backend_lock = backend.read().await;
    if let Some(ref backend) = *backend_lock {
        backend.is_connected().await
    } else {
        false
    }
}

/// Ping the database
pub async fn ping(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
) -> Result<(), DatabaseError> {
    let backend_lock = backend.read().await;
    let backend = backend_lock
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;
    backend.ping().await
}

/// Get current backend type
pub async fn backend_type(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
) -> Option<BackendType> {
    let backend_lock = backend.read().await;
    backend_lock.as_ref().map(|b| b.backend_type())
}

/// Get connection status
pub async fn status(stats: &Arc<RwLock<StorageStats>>) -> ConnectionStatus {
    stats.read().await.status
}

/// Get storage statistics
pub async fn stats(stats: &Arc<RwLock<StorageStats>>) -> StorageStats {
    stats.read().await.clone()
}

/// Get database version
pub async fn version(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
) -> Result<String, DatabaseError> {
    let backend_lock = backend.read().await;
    let backend = backend_lock
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("Not connected".to_string()))?;
    backend.version().await
}

/// Close the connection
pub async fn close(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
) -> Result<(), DatabaseError> {
    let backend_lock = backend.write().await;
    if let Some(ref backend) = *backend_lock {
        backend.close().await?;
    }
    stats.write().await.status = ConnectionStatus::Disconnected;
    Ok(())
}

/// Get health check info
pub async fn health(
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
) -> HealthInfo {
    let stats_snapshot = stats.read().await.clone();
    let connected = is_connected(backend).await;
    let backend_type = backend_type(backend).await;

    HealthInfo {
        connected,
        backend_type,
        status: stats_snapshot.status,
        uptime: stats_snapshot.connected_since.map(|t| Utc::now() - t),
        total_queries: stats_snapshot.total_queries,
        error_rate: if stats_snapshot.total_queries > 0 {
            stats_snapshot.failed_queries as f64 / stats_snapshot.total_queries as f64
        } else {
            0.0
        },
        fallback_count: stats_snapshot.fallback_count,
    }
}
