//! SQL Database Tool
//!
//! This tool provides SQL database operations for multiple database systems:
//! - PostgreSQL
//! - MySQL
//! - SQLite
//! - SQL Server
//!
//! Features:
//! - Query execution
//! - Transaction management
//! - Schema inspection
//! - Data manipulation
//! - Connection pooling

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context, anyhow};
use tracing::{info, debug, error};

use sage_core::tools::{Tool, ToolResult};

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

/// Database tool for SQL operations
#[derive(Debug, Clone)]
pub struct DatabaseTool {
    name: String,
    description: String,
}

impl DatabaseTool {
    /// Create a new database tool
    pub fn new() -> Self {
        Self {
            name: "database".to_string(),
            description: "SQL database operations supporting PostgreSQL, MySQL, SQLite, and SQL Server".to_string(),
        }
    }

    /// Build connection string for different database types
    fn build_connection_string(&self, config: &DatabaseConfig) -> Result<String> {
        match config.database_type {
            DatabaseType::PostgreSQL => {
                if !config.connection_string.starts_with("postgresql://") &&
                   !config.connection_string.starts_with("postgres://") {
                    return Err(anyhow!("PostgreSQL connection string must start with postgresql:// or postgres://"));
                }
                Ok(config.connection_string.clone())
            }
            DatabaseType::MySQL => {
                if !config.connection_string.starts_with("mysql://") {
                    return Err(anyhow!("MySQL connection string must start with mysql://"));
                }
                Ok(config.connection_string.clone())
            }
            DatabaseType::SQLite => {
                // SQLite can be a file path or memory database
                if config.connection_string == ":memory:" {
                    Ok("sqlite::memory:".to_string())
                } else {
                    Ok(format!("sqlite://{}", config.connection_string))
                }
            }
            DatabaseType::SqlServer => {
                if !config.connection_string.starts_with("sqlserver://") &&
                   !config.connection_string.starts_with("mssql://") {
                    return Err(anyhow!("SQL Server connection string must start with sqlserver:// or mssql://"));
                }
                Ok(config.connection_string.clone())
            }
        }
    }

    /// Execute a database operation (mock implementation)
    async fn execute_operation(&self, params: DatabaseParams) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();
        
        debug!("Executing database operation: {:?}", params.operation);
        
        // This is a mock implementation. In a real implementation, you would:
        // 1. Create a connection pool using sqlx or similar
        // 2. Execute the actual SQL operations
        // 3. Return real results
        
        let connection_string = self.build_connection_string(&params.config)?;
        info!("Connecting to database: {}", connection_string);
        
        let result = match params.operation {
            DatabaseOperation::Query { sql, params: _ } => {
                info!("Executing query: {}", sql);
                QueryResult {
                    rows_affected: Some(0),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::Select { sql, params: _, limit } => {
                info!("Executing select: {}", sql);
                // Mock data
                let mut data = vec![
                    HashMap::from([
                        ("id".to_string(), serde_json::json!(1)),
                        ("name".to_string(), serde_json::json!("Sample Record")),
                    ]),
                    HashMap::from([
                        ("id".to_string(), serde_json::json!(2)),
                        ("name".to_string(), serde_json::json!("Another Record")),
                    ]),
                ];
                
                if let Some(limit) = limit {
                    data.truncate(limit);
                }
                
                QueryResult {
                    rows_affected: Some(data.len() as u64),
                    data: Some(data),
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: Some(vec!["id".to_string(), "name".to_string()]),
                }
            }
            DatabaseOperation::Insert { table, data } => {
                info!("Inserting into table: {}", table);
                QueryResult {
                    rows_affected: Some(1),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::Update { table, data, where_clause, params: _ } => {
                info!("Updating table: {} where {}", table, where_clause);
                QueryResult {
                    rows_affected: Some(1),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::Delete { table, where_clause, params: _ } => {
                info!("Deleting from table: {} where {}", table, where_clause);
                QueryResult {
                    rows_affected: Some(1),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::DescribeTable { table } => {
                info!("Describing table: {}", table);
                let schema_data = vec![
                    HashMap::from([
                        ("column_name".to_string(), serde_json::json!("id")),
                        ("data_type".to_string(), serde_json::json!("INTEGER")),
                        ("nullable".to_string(), serde_json::json!(false)),
                        ("primary_key".to_string(), serde_json::json!(true)),
                    ]),
                    HashMap::from([
                        ("column_name".to_string(), serde_json::json!("name")),
                        ("data_type".to_string(), serde_json::json!("VARCHAR(255)")),
                        ("nullable".to_string(), serde_json::json!(true)),
                        ("primary_key".to_string(), serde_json::json!(false)),
                    ]),
                ];
                
                QueryResult {
                    rows_affected: Some(schema_data.len() as u64),
                    data: Some(schema_data),
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: Some(vec!["column_name".to_string(), "data_type".to_string(), "nullable".to_string(), "primary_key".to_string()]),
                }
            }
            DatabaseOperation::ListTables => {
                info!("Listing tables");
                let tables_data = vec![
                    HashMap::from([("table_name".to_string(), serde_json::json!("users"))]),
                    HashMap::from([("table_name".to_string(), serde_json::json!("orders"))]),
                    HashMap::from([("table_name".to_string(), serde_json::json!("products"))]),
                ];
                
                QueryResult {
                    rows_affected: Some(tables_data.len() as u64),
                    data: Some(tables_data),
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: Some(vec!["table_name".to_string()]),
                }
            }
            DatabaseOperation::Transaction { statements } => {
                info!("Executing transaction with {} statements", statements.len());
                QueryResult {
                    rows_affected: Some(statements.len() as u64),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::CreateTable { table, columns } => {
                info!("Creating table: {} with {} columns", table, columns.len());
                QueryResult {
                    rows_affected: Some(1),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::DropTable { table } => {
                info!("Dropping table: {}", table);
                QueryResult {
                    rows_affected: Some(1),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::CreateIndex { table, index_name, columns, unique } => {
                info!("Creating {} index {} on table {} for columns: {}", 
                      if unique { "unique" } else { "non-unique" }, 
                      index_name, table, columns.join(", "));
                QueryResult {
                    rows_affected: Some(1),
                    data: None,
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: None,
                }
            }
            DatabaseOperation::Stats => {
                info!("Getting database statistics");
                let stats_data = vec![
                    HashMap::from([
                        ("metric".to_string(), serde_json::json!("total_tables")),
                        ("value".to_string(), serde_json::json!(3)),
                    ]),
                    HashMap::from([
                        ("metric".to_string(), serde_json::json!("total_rows")),
                        ("value".to_string(), serde_json::json!(1000)),
                    ]),
                    HashMap::from([
                        ("metric".to_string(), serde_json::json!("database_size")),
                        ("value".to_string(), serde_json::json!("10.5 MB")),
                    ]),
                ];
                
                QueryResult {
                    rows_affected: Some(stats_data.len() as u64),
                    data: Some(stats_data),
                    execution_time: start_time.elapsed().as_millis() as u64,
                    columns: Some(vec!["metric".to_string(), "value".to_string()]),
                }
            }
        };
        
        Ok(result)
    }

    /// Format query result for display
    fn format_result(&self, result: &QueryResult) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("Execution time: {}ms\n", result.execution_time));
        
        if let Some(rows_affected) = result.rows_affected {
            output.push_str(&format!("Rows affected: {}\n", rows_affected));
        }
        
        if let Some(data) = &result.data {
            if !data.is_empty() {
                output.push_str("\nResults:\n");
                
                // Get column names
                let columns = if let Some(cols) = &result.columns {
                    cols.clone()
                } else {
                    // Extract column names from first row
                    data.first()
                        .map(|row| row.keys().cloned().collect::<Vec<_>>())
                        .unwrap_or_default()
                };
                
                // Print header
                output.push_str(&format!("| {} |\n", columns.join(" | ")));
                output.push_str(&format!("|{}|\n", 
                    columns.iter().map(|_| "---").collect::<Vec<_>>().join("|")));
                
                // Print rows
                for row in data {
                    let values: Vec<String> = columns.iter()
                        .map(|col| {
                            row.get(col)
                                .map(|v| v.to_string().trim_matches('"').to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        })
                        .collect();
                    output.push_str(&format!("| {} |\n", values.join(" | ")));
                }
            }
        }
        
        output
    }
}

impl Default for DatabaseTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "database_type": {
                            "type": "string",
                            "enum": ["postgresql", "mysql", "sqlite", "sql_server"],
                            "description": "Database type"
                        },
                        "connection_string": {
                            "type": "string",
                            "description": "Database connection string"
                        },
                        "max_connections": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 10,
                            "description": "Maximum number of connections in pool"
                        },
                        "timeout": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 300,
                            "default": 30,
                            "description": "Connection timeout in seconds"
                        }
                    },
                    "required": ["database_type", "connection_string"],
                    "additionalProperties": false
                },
                "operation": {
                    "type": "object",
                    "oneOf": [
                        {
                            "properties": {
                                "query": {
                                    "type": "object",
                                    "properties": {
                                        "sql": { "type": "string" },
                                        "params": {
                                            "type": "array",
                                            "items": {}
                                        }
                                    },
                                    "required": ["sql"]
                                }
                            },
                            "required": ["query"]
                        },
                        {
                            "properties": {
                                "select": {
                                    "type": "object",
                                    "properties": {
                                        "sql": { "type": "string" },
                                        "params": {
                                            "type": "array",
                                            "items": {}
                                        },
                                        "limit": {
                                            "type": "integer",
                                            "minimum": 1
                                        }
                                    },
                                    "required": ["sql"]
                                }
                            },
                            "required": ["select"]
                        },
                        {
                            "properties": {
                                "insert": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" },
                                        "data": {
                                            "type": "object",
                                            "additionalProperties": true
                                        }
                                    },
                                    "required": ["table", "data"]
                                }
                            },
                            "required": ["insert"]
                        },
                        {
                            "properties": {
                                "update": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" },
                                        "data": {
                                            "type": "object",
                                            "additionalProperties": true
                                        },
                                        "where_clause": { "type": "string" },
                                        "params": {
                                            "type": "array",
                                            "items": {}
                                        }
                                    },
                                    "required": ["table", "data", "where_clause"]
                                }
                            },
                            "required": ["update"]
                        },
                        {
                            "properties": {
                                "delete": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" },
                                        "where_clause": { "type": "string" },
                                        "params": {
                                            "type": "array",
                                            "items": {}
                                        }
                                    },
                                    "required": ["table", "where_clause"]
                                }
                            },
                            "required": ["delete"]
                        },
                        {
                            "properties": {
                                "describe_table": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" }
                                    },
                                    "required": ["table"]
                                }
                            },
                            "required": ["describe_table"]
                        },
                        {
                            "properties": {
                                "list_tables": { "type": "null" }
                            },
                            "required": ["list_tables"]
                        },
                        {
                            "properties": {
                                "transaction": {
                                    "type": "object",
                                    "properties": {
                                        "statements": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["statements"]
                                }
                            },
                            "required": ["transaction"]
                        },
                        {
                            "properties": {
                                "create_table": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" },
                                        "columns": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "name": { "type": "string" },
                                                    "data_type": { "type": "string" },
                                                    "nullable": { "type": "boolean" },
                                                    "primary_key": { "type": "boolean" },
                                                    "default_value": { "type": "string" }
                                                },
                                                "required": ["name", "data_type", "nullable", "primary_key"]
                                            }
                                        }
                                    },
                                    "required": ["table", "columns"]
                                }
                            },
                            "required": ["create_table"]
                        },
                        {
                            "properties": {
                                "drop_table": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" }
                                    },
                                    "required": ["table"]
                                }
                            },
                            "required": ["drop_table"]
                        },
                        {
                            "properties": {
                                "create_index": {
                                    "type": "object",
                                    "properties": {
                                        "table": { "type": "string" },
                                        "index_name": { "type": "string" },
                                        "columns": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "unique": { "type": "boolean" }
                                    },
                                    "required": ["table", "index_name", "columns", "unique"]
                                }
                            },
                            "required": ["create_index"]
                        },
                        {
                            "properties": {
                                "stats": { "type": "null" }
                            },
                            "required": ["stats"]
                        }
                    ]
                }
            },
            "required": ["config", "operation"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: DatabaseParams = serde_json::from_value(params)
            .context("Failed to parse database parameters")?;

        info!("Executing database operation: {:?}", params.operation);

        let result = self.execute_operation(params).await?;
        let formatted_result = self.format_result(&result);
        
        let mut metadata = HashMap::new();
        metadata.insert("execution_time".to_string(), format!("{}ms", result.execution_time));
        
        if let Some(rows_affected) = result.rows_affected {
            metadata.insert("rows_affected".to_string(), rows_affected.to_string());
        }
        
        if let Some(columns) = &result.columns {
            metadata.insert("columns".to_string(), columns.join(", "));
        }

        Ok(ToolResult::new(formatted_result, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_database_tool_creation() {
        let tool = DatabaseTool::new();
        assert_eq!(tool.name(), "database");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_connection_string_building() {
        let tool = DatabaseTool::new();
        
        let pg_config = DatabaseConfig {
            database_type: DatabaseType::PostgreSQL,
            connection_string: "postgresql://user:pass@localhost/db".to_string(),
            max_connections: None,
            timeout: None,
        };
        
        let result = tool.build_connection_string(&pg_config).unwrap();
        assert_eq!(result, "postgresql://user:pass@localhost/db");
        
        let sqlite_config = DatabaseConfig {
            database_type: DatabaseType::SQLite,
            connection_string: "/path/to/db.sqlite".to_string(),
            max_connections: None,
            timeout: None,
        };
        
        let result = tool.build_connection_string(&sqlite_config).unwrap();
        assert_eq!(result, "sqlite:///path/to/db.sqlite");
    }

    #[tokio::test]
    async fn test_database_tool_schema() {
        let tool = DatabaseTool::new();
        let schema = tool.parameters_json_schema();
        
        assert!(schema.is_object());
        assert!(schema["properties"]["config"].is_object());
        assert!(schema["properties"]["operation"].is_object());
    }
}