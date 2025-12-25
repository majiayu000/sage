//! Schema definition for the AskUserQuestion tool

use sage_core::tools::types::ToolSchema;
use serde_json::json;

/// Creates the JSON schema for the AskUserQuestion tool
pub fn create_schema(name: &str, description: &str) -> ToolSchema {
    ToolSchema {
        name: name.to_string(),
        description: description.to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "Array of 1-4 questions to ask the user",
                    "minItems": 1,
                    "maxItems": 4,
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The question text to ask the user"
                            },
                            "header": {
                                "type": "string",
                                "description": "Short label for the question (max 12 chars) like 'Auth method', 'Library', 'Framework'",
                                "maxLength": 12
                            },
                            "options": {
                                "type": "array",
                                "description": "Array of 2-4 options for the user to choose from",
                                "minItems": 2,
                                "maxItems": 4,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "Display text for this option"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "Explanation of what this option means or does"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multi_select": {
                                "type": "boolean",
                                "description": "Whether multiple options can be selected. Defaults to false.",
                                "default": false
                            }
                        },
                        "required": ["question", "header", "options"]
                    }
                },
                "answers": {
                    "type": "object",
                    "description": "Optional: Previously collected answers for processing. The agent should not provide this on first call.",
                    "additionalProperties": true
                }
            },
            "required": ["questions"]
        }),
    }
}
