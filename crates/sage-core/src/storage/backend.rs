//! Database backend implementations
//!
//! Provides a unified interface for different database backends with
//! SQLite and PostgreSQL implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Database error types
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Query failed: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Constraint violation: {0}")]
    Constraint(String),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("Backend not available: {0}")]
    NotAvailable(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Database backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    PostgreSQL,
    SQLite,
    InMemory,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PostgreSQL => write!(f, "PostgreSQL"),
            Self::SQLite => write!(f, "SQLite"),
            Self::InMemory => write!(f, "InMemory"),
        }
    }
}

/// Database value for dynamic typing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Timestamp(chrono::DateTime<chrono::Utc>),
}

impl DatabaseValue {
    /// Check if null
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Try to get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            Self::Int(i) => Some(*i != 0),
            _ => None,
        }
    }

    /// Try to get as JSON
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Json(j) => Some(j),
            _ => None,
        }
    }
}

impl From<String> for DatabaseValue {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for DatabaseValue {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<i64> for DatabaseValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<i32> for DatabaseValue {
    fn from(i: i32) -> Self {
        Self::Int(i as i64)
    }
}

impl From<f64> for DatabaseValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for DatabaseValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<serde_json::Value> for DatabaseValue {
    fn from(j: serde_json::Value) -> Self {
        Self::Json(j)
    }
}

/// A row from a query result
#[derive(Debug, Clone, Default)]
pub struct DatabaseRow {
    columns: HashMap<String, DatabaseValue>,
}

impl DatabaseRow {
    /// Create empty row
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from column map
    pub fn from_map(columns: HashMap<String, DatabaseValue>) -> Self {
        Self { columns }
    }

    /// Get a value by column name
    pub fn get(&self, column: &str) -> Option<&DatabaseValue> {
        self.columns.get(column)
    }

    /// Get string value
    pub fn get_str(&self, column: &str) -> Option<&str> {
        self.get(column).and_then(|v| v.as_str())
    }

    /// Get i64 value
    pub fn get_i64(&self, column: &str) -> Option<i64> {
        self.get(column).and_then(|v| v.as_i64())
    }

    /// Get f64 value
    pub fn get_f64(&self, column: &str) -> Option<f64> {
        self.get(column).and_then(|v| v.as_f64())
    }

    /// Get bool value
    pub fn get_bool(&self, column: &str) -> Option<bool> {
        self.get(column).and_then(|v| v.as_bool())
    }

    /// Get JSON value
    pub fn get_json(&self, column: &str) -> Option<&serde_json::Value> {
        self.get(column).and_then(|v| v.as_json())
    }

    /// Set a column value
    pub fn set(&mut self, column: impl Into<String>, value: impl Into<DatabaseValue>) {
        self.columns.insert(column.into(), value.into());
    }

    /// Get all column names
    pub fn columns(&self) -> impl Iterator<Item = &str> {
        self.columns.keys().map(|s| s.as_str())
    }

    /// Check if column exists
    pub fn has_column(&self, column: &str) -> bool {
        self.columns.contains_key(column)
    }
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Rows affected (for INSERT/UPDATE/DELETE)
    pub rows_affected: u64,
    /// Returned rows (for SELECT)
    pub rows: Vec<DatabaseRow>,
    /// Last insert ID (if applicable)
    pub last_insert_id: Option<i64>,
}

impl QueryResult {
    /// Create empty result
    pub fn empty() -> Self {
        Self {
            rows_affected: 0,
            rows: Vec::new(),
            last_insert_id: None,
        }
    }

    /// Create from rows
    pub fn from_rows(rows: Vec<DatabaseRow>) -> Self {
        Self {
            rows_affected: rows.len() as u64,
            rows,
            last_insert_id: None,
        }
    }

    /// Create from affected count
    pub fn from_affected(count: u64) -> Self {
        Self {
            rows_affected: count,
            rows: Vec::new(),
            last_insert_id: None,
        }
    }

    /// Check if result is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get first row
    pub fn first(&self) -> Option<&DatabaseRow> {
        self.rows.first()
    }

    /// Get row count
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

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
    async fn query(&self, sql: &str, params: &[DatabaseValue]) -> Result<QueryResult, DatabaseError>;

    /// Execute a statement (INSERT/UPDATE/DELETE)
    async fn execute(&self, sql: &str, params: &[DatabaseValue]) -> Result<QueryResult, DatabaseError>;

    /// Execute multiple statements in a transaction
    async fn transaction(&self, statements: Vec<(&str, Vec<DatabaseValue>)>) -> Result<(), DatabaseError>;

    /// Get database version
    async fn version(&self) -> Result<String, DatabaseError>;

    /// Close the connection
    async fn close(&self) -> Result<(), DatabaseError>;
}

/// SQLite backend implementation
pub struct SqliteBackend {
    path: String,
    // In a real implementation, this would use rusqlite or sqlx
    // For now, we use an in-memory simulation
    data: Arc<RwLock<HashMap<String, Vec<DatabaseRow>>>>,
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

        Ok(Self {
            path: path_str,
            data: Arc::new(RwLock::new(HashMap::new())),
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

    /// Extract column names from INSERT statement
    /// Parses: INSERT INTO table (col1, col2, col3) VALUES (?, ?, ?)
    fn extract_insert_columns(&self, sql: &str) -> Vec<String> {
        // Find the column list between first ( and ) before VALUES
        let sql_lower = sql.to_lowercase();

        // Find the opening parenthesis after table name
        if let Some(open_paren) = sql.find('(') {
            // Find VALUES keyword
            let values_pos = sql_lower.find("values").unwrap_or(sql.len());

            // The column list should be between first ( and the ) before VALUES
            if open_paren < values_pos {
                // Find closing paren for column list
                let after_open = &sql[open_paren + 1..];
                if let Some(close_paren) = after_open.find(')') {
                    let column_str = &after_open[..close_paren];

                    // Parse column names
                    return column_str
                        .split(',')
                        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '`').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }

        Vec::new()
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

    async fn query(&self, sql: &str, params: &[DatabaseValue]) -> Result<QueryResult, DatabaseError> {
        if !self.is_connected().await {
            return Err(DatabaseError::Connection("Not connected".to_string()));
        }

        let sql_lower = sql.to_lowercase();
        if sql_lower.contains("select") && sql_lower.contains("from") {
            // Extract table name
            if let Some(from_pos) = sql_lower.find("from") {
                let after_from = &sql_lower[from_pos + 5..];
                let table_name = after_from
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '"' || c == '`' || c == ';');

                let data = self.data.read().await;
                if let Some(rows) = data.get(table_name) {
                    // Check for WHERE clause with key parameter
                    if sql_lower.contains("where") && !params.is_empty() {
                        // Find rows matching the key (first param)
                        if let DatabaseValue::Text(key) = &params[0] {
                            let matching: Vec<DatabaseRow> = rows
                                .iter()
                                .filter(|r| {
                                    r.get("key").and_then(|v| v.as_str()) == Some(key.as_str())
                                })
                                .cloned()
                                .collect();
                            return Ok(QueryResult::from_rows(matching));
                        }
                    }

                    // Check for ORDER BY ... LIMIT 1 (for schema version)
                    if sql_lower.contains("limit 1") {
                        if let Some(first) = rows.last() {
                            return Ok(QueryResult::from_rows(vec![first.clone()]));
                        }
                    }

                    return Ok(QueryResult::from_rows(rows.clone()));
                }
            }
        }

        Ok(QueryResult::empty())
    }

    async fn execute(&self, sql: &str, params: &[DatabaseValue]) -> Result<QueryResult, DatabaseError> {
        if !self.is_connected().await {
            return Err(DatabaseError::Connection("Not connected".to_string()));
        }

        let sql_lower = sql.to_lowercase();

        // Handle CREATE TABLE
        if sql_lower.starts_with("create table") {
            // Extract table name
            if let Some(start) = sql_lower.find("create table") {
                let after = &sql[start + 12..];
                let table_name = after
                    .trim()
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '"' || c == '`')
                    .to_string();

                if !table_name.is_empty() {
                    let mut data = self.data.write().await;
                    data.entry(table_name).or_insert_with(Vec::new);
                }
            }
            return Ok(QueryResult::from_affected(0));
        }

        // Handle INSERT
        if sql_lower.starts_with("insert") {
            // Extract table name and column names
            if let Some(into_pos) = sql_lower.find("into") {
                let after_into = &sql[into_pos + 4..];
                let table_name = after_into
                    .trim()
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '"' || c == '`')
                    .to_string();

                if !table_name.is_empty() {
                    // Extract column names from INSERT INTO table (col1, col2, ...) VALUES (?, ?)
                    let column_names = self.extract_insert_columns(sql);

                    let mut data = self.data.write().await;
                    let table = data.entry(table_name).or_insert_with(Vec::new);

                    // Create row from params with proper column names
                    let mut row = DatabaseRow::new();
                    for (i, param) in params.iter().enumerate() {
                        let col_name = column_names.get(i)
                            .map(|s| s.as_str())
                            .unwrap_or_else(|| {
                                // Fallback to generic names
                                match i {
                                    0 => "col0",
                                    1 => "col1",
                                    2 => "col2",
                                    3 => "col3",
                                    _ => "col_unknown",
                                }
                            });
                        row.set(col_name, param.clone());
                    }
                    table.push(row);

                    return Ok(QueryResult {
                        rows_affected: 1,
                        rows: Vec::new(),
                        last_insert_id: Some(table.len() as i64),
                    });
                }
            }
        }

        // Handle DELETE
        if sql_lower.starts_with("delete") {
            if let Some(from_pos) = sql_lower.find("from") {
                let after_from = &sql[from_pos + 4..];
                let table_name = after_from
                    .trim()
                    .split(|c: char| c.is_whitespace())
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '"' || c == '`')
                    .to_string();

                if !table_name.is_empty() {
                    let mut data = self.data.write().await;
                    if let Some(table) = data.get_mut(&table_name) {
                        // Check for WHERE clause with key
                        if sql_lower.contains("where") && !params.is_empty() {
                            if let DatabaseValue::Text(key) = &params[0] {
                                let original_len = table.len();
                                table.retain(|row| {
                                    row.get("key").and_then(|v| v.as_str()) != Some(key.as_str())
                                });
                                let deleted = original_len - table.len();
                                return Ok(QueryResult::from_affected(deleted as u64));
                            }
                            // Also check for version column (for schema_migrations)
                            if let DatabaseValue::Int(version) = &params[0] {
                                let original_len = table.len();
                                table.retain(|row| {
                                    row.get("version").and_then(|v| v.as_i64()) != Some(*version)
                                });
                                let deleted = original_len - table.len();
                                return Ok(QueryResult::from_affected(deleted as u64));
                            }
                        }
                    }
                }
            }
        }

        Ok(QueryResult::from_affected(0))
    }

    async fn transaction(&self, statements: Vec<(&str, Vec<DatabaseValue>)>) -> Result<(), DatabaseError> {
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

/// PostgreSQL backend implementation (stub)
pub struct PostgresBackend {
    connection_string: String,
    connected: Arc<RwLock<bool>>,
}

impl PostgresBackend {
    /// Try to connect to PostgreSQL
    pub async fn connect(connection_string: &str) -> Result<Self, DatabaseError> {
        tracing::info!("PostgreSQL backend attempting connection to: {}", connection_string);

        // In a real implementation, would use tokio-postgres or sqlx
        // For now, simulate connection failure to demonstrate fallback

        // Check if connection string looks valid
        if !connection_string.starts_with("postgresql://") && !connection_string.starts_with("postgres://") {
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

    async fn query(&self, _sql: &str, _params: &[DatabaseValue]) -> Result<QueryResult, DatabaseError> {
        Err(DatabaseError::NotAvailable("PostgreSQL not implemented".to_string()))
    }

    async fn execute(&self, _sql: &str, _params: &[DatabaseValue]) -> Result<QueryResult, DatabaseError> {
        Err(DatabaseError::NotAvailable("PostgreSQL not implemented".to_string()))
    }

    async fn transaction(&self, _statements: Vec<(&str, Vec<DatabaseValue>)>) -> Result<(), DatabaseError> {
        Err(DatabaseError::NotAvailable("PostgreSQL not implemented".to_string()))
    }

    async fn version(&self) -> Result<String, DatabaseError> {
        Err(DatabaseError::NotAvailable("PostgreSQL not implemented".to_string()))
    }

    async fn close(&self) -> Result<(), DatabaseError> {
        *self.connected.write().await = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_value_conversions() {
        let v: DatabaseValue = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));

        let v: DatabaseValue = 42i64.into();
        assert_eq!(v.as_i64(), Some(42));

        let v: DatabaseValue = 3.14f64.into();
        assert_eq!(v.as_f64(), Some(3.14));

        let v: DatabaseValue = true.into();
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_database_row() {
        let mut row = DatabaseRow::new();
        row.set("name", "Alice");
        row.set("age", 30i64);
        row.set("active", true);

        assert_eq!(row.get_str("name"), Some("Alice"));
        assert_eq!(row.get_i64("age"), Some(30));
        assert_eq!(row.get_bool("active"), Some(true));
        assert!(row.has_column("name"));
        assert!(!row.has_column("missing"));
    }

    #[test]
    fn test_query_result() {
        let result = QueryResult::empty();
        assert!(result.is_empty());

        let mut row = DatabaseRow::new();
        row.set("id", 1i64);
        let result = QueryResult::from_rows(vec![row]);
        assert_eq!(result.len(), 1);
        assert!(result.first().is_some());
    }

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

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::PostgreSQL.to_string(), "PostgreSQL");
        assert_eq!(BackendType::SQLite.to_string(), "SQLite");
        assert_eq!(BackendType::InMemory.to_string(), "InMemory");
    }
}
