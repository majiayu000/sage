//! Query operation schemas (SELECT, raw SQL)

/// Schemas for query operations (SELECT, raw SQL)
pub fn query_operation_schemas() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
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
        }),
        serde_json::json!({
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
        }),
    ]
}
