//! Tool parameter and schema types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameter definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Parameter type (string, number, boolean, object, array)
    pub param_type: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value (if any)
    pub default: Option<serde_json::Value>,
    /// Enum values (if applicable)
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// Additional schema properties
    pub properties: HashMap<String, serde_json::Value>,
}

impl ToolParameter {
    /// Create a required string parameter
    pub fn string<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: true,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Create an optional string parameter
    pub fn optional_string<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: false,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Create a boolean parameter
    pub fn boolean<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "boolean".to_string(),
            required: true,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Create a number parameter
    pub fn number<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "number".to_string(),
            required: true,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Make parameter optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set default value
    pub fn with_default<V: Into<serde_json::Value>>(mut self, default: V) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// JSON schema for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input parameters schema
    pub parameters: serde_json::Value,
}

impl ToolSchema {
    /// Create a new tool schema
    pub fn new<S: Into<String>>(name: S, description: S, parameters: Vec<ToolParameter>) -> Self {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in parameters {
            if param.required {
                required.push(param.name.clone());
            }

            let mut param_schema = serde_json::Map::new();
            param_schema.insert("type".to_string(), param.param_type.into());
            param_schema.insert("description".to_string(), param.description.into());

            if let Some(default) = param.default {
                param_schema.insert("default".to_string(), default);
            }

            if let Some(enum_values) = param.enum_values {
                param_schema.insert("enum".to_string(), enum_values.into());
            }

            for (key, value) in param.properties {
                param_schema.insert(key, value);
            }

            properties.insert(param.name, param_schema.into());
        }

        let parameters_schema = serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        });

        Self {
            name: name.into(),
            description: description.into(),
            parameters: parameters_schema,
        }
    }

    /// Create a flexible tool schema with custom parameters JSON
    pub fn new_flexible<S: Into<String>>(
        name: S,
        description: S,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}
