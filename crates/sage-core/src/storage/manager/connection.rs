//! Database connection establishment and fallback logic

use super::types::{ConnectionStatus, StorageStats};
use crate::storage::backend::{
    BackendType, DatabaseBackend, DatabaseError, PostgresBackend, SqliteBackend,
};
use crate::storage::config::{FallbackStrategy, StorageConfig};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Establish database connection with fallback logic
pub async fn establish_connection(
    config: &StorageConfig,
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
) -> Result<(), DatabaseError> {
    match config.fallback_strategy {
        FallbackStrategy::SqliteOnly => connect_sqlite(config, backend, stats).await,
        FallbackStrategy::FailFast => {
            if config.should_try_postgres() {
                connect_postgres(config, backend, stats).await
            } else {
                connect_sqlite(config, backend, stats).await
            }
        }
        FallbackStrategy::AutoFallback => {
            if config.should_try_postgres() {
                match connect_postgres(config, backend, stats).await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        tracing::warn!(
                            "PostgreSQL connection failed: {}. Falling back to SQLite.",
                            e
                        );
                        stats.write().await.fallback_count += 1;
                        connect_sqlite(config, backend, stats).await
                    }
                }
            } else {
                connect_sqlite(config, backend, stats).await
            }
        }
        FallbackStrategy::RetryThenFallback => {
            if config.should_try_postgres() {
                match connect_postgres_with_retry(config, backend, stats).await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        tracing::warn!(
                            "PostgreSQL connection failed after retries: {}. Falling back to SQLite.",
                            e
                        );
                        stats.write().await.fallback_count += 1;
                        connect_sqlite(config, backend, stats).await
                    }
                }
            } else {
                connect_sqlite(config, backend, stats).await
            }
        }
    }
}

/// Connect to PostgreSQL
pub async fn connect_postgres(
    config: &StorageConfig,
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
) -> Result<(), DatabaseError> {
    let pg_config = config
        .postgres
        .as_ref()
        .ok_or_else(|| DatabaseError::Connection("PostgreSQL not configured".to_string()))?;

    tracing::info!("Attempting PostgreSQL connection...");

    let pg_backend = PostgresBackend::connect(&pg_config.connection_string).await?;

    *backend.write().await = Some(Box::new(pg_backend));

    let mut stats = stats.write().await;
    stats.backend_type = Some(BackendType::PostgreSQL);
    stats.status = ConnectionStatus::Primary;
    stats.connected_since = Some(Utc::now());

    tracing::info!("Connected to PostgreSQL successfully");
    Ok(())
}

/// Connect to PostgreSQL with retry
pub async fn connect_postgres_with_retry(
    config: &StorageConfig,
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
) -> Result<(), DatabaseError> {
    let mut delay = config.retry.initial_delay;
    let mut last_error = None;

    for attempt in 1..=config.retry.max_attempts {
        tracing::info!(
            "PostgreSQL connection attempt {}/{}",
            attempt,
            config.retry.max_attempts
        );

        match connect_postgres(config, backend, stats).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);

                if attempt < config.retry.max_attempts {
                    tracing::warn!("Connection failed, retrying in {:?}...", delay);
                    sleep(delay).await;

                    // Exponential backoff
                    delay = Duration::from_secs_f64(
                        delay.as_secs_f64() * config.retry.backoff_multiplier,
                    )
                    .min(config.retry.max_delay);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| DatabaseError::Connection("Max retries exceeded".to_string())))
}

/// Connect to SQLite
pub async fn connect_sqlite(
    config: &StorageConfig,
    backend: &Arc<RwLock<Option<Box<dyn DatabaseBackend>>>>,
    stats: &Arc<RwLock<StorageStats>>,
) -> Result<(), DatabaseError> {
    tracing::info!("Connecting to SQLite at: {:?}", config.sqlite.path);

    let sqlite_backend = SqliteBackend::connect(&config.sqlite.path).await?;
    let backend_type = sqlite_backend.backend_type();

    *backend.write().await = Some(Box::new(sqlite_backend));

    let mut stats = stats.write().await;
    stats.backend_type = Some(backend_type);
    stats.status = ConnectionStatus::Fallback;
    stats.connected_since = Some(Utc::now());

    tracing::info!("Connected to SQLite successfully");
    Ok(())
}
