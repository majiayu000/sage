//! Database backend trait definition
//!
//! Defines the common interface for all database backend implementations.

use async_trait::async_trait;

use super::types::{BackendType, DatabaseError, DatabaseValue, QueryResult};

/// Database backend trait
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Get backend type
    fn backend_type(&self) -> BackendType;

    /// Check if connected
    async fn is_connected(&self) -> bool;

    /// Ping the database
    async fn ping(&self) -> Result<(), DatabaseError>;

    /// Execute a query (SELECT)
    async fn query(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError>;

    /// Execute a statement (INSERT/UPDATE/DELETE)
    async fn execute(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError>;

    /// Execute multiple statements in a transaction
    async fn transaction(
        &self,
        statements: Vec<(&str, Vec<DatabaseValue>)>,
    ) -> Result<(), DatabaseError>;

    /// Get database version
    async fn version(&self) -> Result<String, DatabaseError>;

    /// Close the connection
    async fn close(&self) -> Result<(), DatabaseError>;
}
