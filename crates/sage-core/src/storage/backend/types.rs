//! Database type definitions
//!
//! Provides core types for database operations including values, rows,
//! results, and error types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

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

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::PostgreSQL.to_string(), "PostgreSQL");
        assert_eq!(BackendType::SQLite.to_string(), "SQLite");
        assert_eq!(BackendType::InMemory.to_string(), "InMemory");
    }
}
