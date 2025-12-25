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

mod types;
pub(crate) mod schema;
mod validation;
pub(crate) mod execution;
mod tool;

// Re-export public types
pub use types::{
    DatabaseType,
    DatabaseConfig,
    DatabaseOperation,
    ColumnDefinition,
    DatabaseParams,
    QueryResult,
};

// Re-export the main tool
pub use tool::DatabaseTool;

// Re-export execution functions for advanced usage
pub use execution::{execute_operation, format_result};

// Re-export validation functions for advanced usage
pub use validation::build_connection_string;
