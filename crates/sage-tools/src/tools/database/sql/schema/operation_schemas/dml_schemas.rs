//! DML operation schemas (INSERT, UPDATE, DELETE, TRANSACTION)

/// Schemas for DML operations (INSERT, UPDATE, DELETE)
pub fn dml_operation_schemas() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
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
        }),
        serde_json::json!({
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
        }),
        serde_json::json!({
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
        }),
        serde_json::json!({
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
        }),
    ]
}
