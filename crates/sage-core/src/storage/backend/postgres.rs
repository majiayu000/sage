//! PostgreSQL backend implementation
//!
//! Provides PostgreSQL database backend (stub implementation).

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::r#trait::DatabaseBackend;
use super::types::{BackendType, DatabaseError, DatabaseValue, QueryResult};

/// PostgreSQL backend implementation (stub)
pub struct PostgresBackend {
    connection_string: String,
    connected: Arc<RwLock<bool>>,
}

impl PostgresBackend {
    /// Try to connect to PostgreSQL
    pub async fn connect(connection_string: &str) -> Result<Self, DatabaseError> {
        tracing::info!(
            "PostgreSQL backend attempting connection to: {}",
            connection_string
        );

        // In a real implementation, would use tokio-postgres or sqlx
        // For now, simulate connection failure to demonstrate fallback

        // Check if connection string looks valid
        if !connection_string.starts_with("postgresql://")
            && !connection_string.starts_with("postgres://")
        {
            return Err(DatabaseError::Connection(format!(
                "Invalid connection string: {}",
                connection_string
            )));
        }

        // Simulate connection attempt
        // In production, would actually try to connect here

        // For demonstration, fail if no actual driver is available
        // This simulates the error: "has no supporting driver"
        Err(DatabaseError::NotAvailable(format!(
            "PostgreSQL driver not available. Connection string '{}' has no supporting driver. \
            Falling back to SQLite.",
            connection_string
        )))
    }

    /// Get connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }
}

#[async_trait]
impl DatabaseBackend for PostgresBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::PostgreSQL
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
        _sql: &str,
        _params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        Err(DatabaseError::NotAvailable(
            "PostgreSQL not implemented".to_string(),
        ))
    }

    async fn execute(
        &self,
        _sql: &str,
        _params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        Err(DatabaseError::NotAvailable(
            "PostgreSQL not implemented".to_string(),
        ))
    }

    async fn transaction(
        &self,
        _statements: Vec<(&str, Vec<DatabaseValue>)>,
    ) -> Result<(), DatabaseError> {
        Err(DatabaseError::NotAvailable(
            "PostgreSQL not implemented".to_string(),
        ))
    }

    async fn version(&self) -> Result<String, DatabaseError> {
        Err(DatabaseError::NotAvailable(
            "PostgreSQL not implemented".to_string(),
        ))
    }

    async fn close(&self) -> Result<(), DatabaseError> {
        *self.connected.write().await = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_postgres_backend_connection_fails() {
        // Should fail because no driver is available
        let result = PostgresBackend::connect("postgresql://localhost/test").await;
        assert!(result.is_err());

        if let Err(DatabaseError::NotAvailable(msg)) = result {
            assert!(msg.contains("driver not available"));
        } else {
            panic!("Expected NotAvailable error");
        }
    }
}
