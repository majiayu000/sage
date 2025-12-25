//! Tests for SQLite backend

use super::backend::SqliteBackend;
use crate::storage::backend::r#trait::DatabaseBackend;
use crate::storage::backend::types::BackendType;

#[tokio::test]
async fn test_sqlite_backend_connect() {
    let backend = SqliteBackend::in_memory().await.unwrap();
    assert!(backend.is_connected().await);
    assert_eq!(backend.backend_type(), BackendType::InMemory);
}

#[tokio::test]
async fn test_sqlite_backend_ping() {
    let backend = SqliteBackend::in_memory().await.unwrap();
    assert!(backend.ping().await.is_ok());
}

#[tokio::test]
async fn test_sqlite_backend_execute() {
    let backend = SqliteBackend::in_memory().await.unwrap();

    // Create table
    let result = backend
        .execute("CREATE TABLE users (id INTEGER, name TEXT)", &[])
        .await
        .unwrap();
    assert_eq!(result.rows_affected, 0);

    // Insert
    let result = backend
        .execute(
            "INSERT INTO users VALUES (?, ?)",
            &[1i64.into(), "Alice".into()],
        )
        .await
        .unwrap();
    assert_eq!(result.rows_affected, 1);
}

#[tokio::test]
async fn test_sqlite_backend_close() {
    let backend = SqliteBackend::in_memory().await.unwrap();
    assert!(backend.is_connected().await);

    backend.close().await.unwrap();
    assert!(!backend.is_connected().await);
}
