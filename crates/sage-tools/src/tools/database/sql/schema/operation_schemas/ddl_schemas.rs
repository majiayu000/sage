//! DDL operation schemas (CREATE TABLE, DROP TABLE, CREATE INDEX)

/// Schemas for DDL operations (CREATE, DROP, ALTER)
pub fn ddl_operation_schemas() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
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
        }),
        serde_json::json!({
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
        }),
        serde_json::json!({
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
        }),
    ]
}
