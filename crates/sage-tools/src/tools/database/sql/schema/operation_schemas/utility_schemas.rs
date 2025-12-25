//! Utility operation schemas (DESCRIBE, LIST TABLES, STATS)

/// Schemas for utility operations (DESCRIBE, LIST, STATS)
pub fn utility_operation_schemas() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
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
        }),
        serde_json::json!({
            "properties": {
                "list_tables": { "type": "null" }
            },
            "required": ["list_tables"]
        }),
        serde_json::json!({
            "properties": {
                "stats": { "type": "null" }
            },
            "required": ["stats"]
        }),
    ]
}
