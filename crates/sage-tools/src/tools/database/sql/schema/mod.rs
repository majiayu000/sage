//! JSON Schema Definitions for SQL Database Tool

pub(crate) mod operation_schemas;

use operation_schemas::{
    config_schema,
    query_operation_schemas,
    ddl_operation_schemas,
    dml_operation_schemas,
    utility_operation_schemas,
};

/// Generate JSON schema for database tool parameters
pub fn parameters_json_schema() -> serde_json::Value {
    let mut one_of = Vec::new();
    one_of.extend(query_operation_schemas());
    one_of.extend(dml_operation_schemas());
    one_of.extend(ddl_operation_schemas());
    one_of.extend(utility_operation_schemas());

    serde_json::json!({
        "type": "object",
        "properties": {
            "config": config_schema(),
            "operation": {
                "type": "object",
                "oneOf": one_of
            }
        },
        "required": ["config", "operation"],
        "additionalProperties": false
    })
}
