//! Operation-specific JSON schemas

mod query_schemas;
mod dml_schemas;
mod ddl_schemas;
mod utility_schemas;

pub use query_schemas::query_operation_schemas;
pub use dml_schemas::dml_operation_schemas;
pub use ddl_schemas::ddl_operation_schemas;
pub use utility_schemas::utility_operation_schemas;

/// Schema for database configuration
pub fn config_schema() -> serde_json::Value {
    serde_json::json!({
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
    })
}
