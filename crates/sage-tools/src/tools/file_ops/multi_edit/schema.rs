//! Tool schema definition for multi-edit

use sage_core::tools::types::{ToolParameter, ToolSchema};
use std::collections::HashMap;

/// Create the tool schema for MultiEdit
pub fn create_schema(name: &str, description: &str) -> ToolSchema {
    // Create the items schema for the edits array as serde_json::Value
    let items_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "old_string": {
                "type": "string",
                "description": "The text to replace"
            },
            "new_string": {
                "type": "string",
                "description": "The replacement text"
            },
            "replace_all": {
                "type": "boolean",
                "description": "Replace all occurrences (default: false)",
                "default": false
            }
        },
        "required": ["old_string", "new_string"]
    });

    let mut edits_properties = HashMap::new();
    edits_properties.insert("items".to_string(), items_schema);

    ToolSchema::new(
        name,
        description,
        vec![
            ToolParameter::string(
                "file_path",
                "The absolute path to the file to edit (must be absolute, not relative)",
            ),
            ToolParameter {
                name: "edits".to_string(),
                description: "Array of edit operations".to_string(),
                param_type: "array".to_string(),
                required: true,
                default: None,
                enum_values: None,
                properties: edits_properties,
            },
        ],
    )
}
