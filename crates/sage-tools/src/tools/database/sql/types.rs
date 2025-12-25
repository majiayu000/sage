//! SQL Database Type Definitions

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Supported database types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
    SqlServer,
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type
    pub database_type: DatabaseType,
    /// Connection string or file path (for SQLite)
    pub connection_string: String,
    /// Maximum number of connections in pool
    pub max_connections: Option<u32>,
    /// Connection timeout in seconds
    pub timeout: Option<u64>,
}

/// Database operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseOperation {
    /// Execute a query
    Query {
        sql: String,
        params: Option<Vec<serde_json::Value>>,
    },
    /// Execute a query and return results
    Select {
        sql: String,
        params: Option<Vec<serde_json::Value>>,
        limit: Option<usize>,
    },
    /// Execute an insert statement
    Insert {
        table: String,
        data: HashMap<String, serde_json::Value>,
    },
    /// Execute an update statement
    Update {
        table: String,
        data: HashMap<String, serde_json::Value>,
        where_clause: String,
        params: Option<Vec<serde_json::Value>>,
    },
    /// Execute a delete statement
    Delete {
        table: String,
        where_clause: String,
        params: Option<Vec<serde_json::Value>>,
    },
    /// Get table schema
    DescribeTable {
        table: String,
    },
    /// List all tables
    ListTables,
    /// Execute multiple statements in a transaction
    Transaction {
        statements: Vec<String>,
    },
    /// Create a table
    CreateTable {
        table: String,
        columns: Vec<ColumnDefinition>,
    },
    /// Drop a table
    DropTable {
        table: String,
    },
    /// Create an index
    CreateIndex {
        table: String,
        index_name: String,
        columns: Vec<String>,
        unique: bool,
    },
    /// Show database statistics
    Stats,
}

/// Column definition for table creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub default_value: Option<String>,
}

/// Database tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseParams {
    /// Database configuration
    pub config: DatabaseConfig,
    /// Database operation
    pub operation: DatabaseOperation,
}

/// Database query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Number of rows affected
    pub rows_affected: Option<u64>,
    /// Result data
    pub data: Option<Vec<HashMap<String, serde_json::Value>>>,
    /// Execution time in milliseconds
    pub execution_time: u64,
    /// Column names
    pub columns: Option<Vec<String>>,
}
