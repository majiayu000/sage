//! JSON Schema conversion utilities and validation

use super::translator::SchemaTranslator;
use crate::tools::types::ToolParameter;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use tracing::warn;

impl SchemaTranslator {
    // ==========================================================================
    // JSON Schema Utilities
    // ==========================================================================

    /// Extract parameters from a JSON Schema
    pub fn extract_parameters_from_schema(schema: &Value) -> Vec<ToolParameter> {
        let mut parameters = Vec::new();

        if let Value::Object(obj) = schema {
            let properties = obj.get("properties").and_then(|p| p.as_object());
            let required = obj
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            if let Some(props) = properties {
                for (name, prop) in props {
                    let is_required = required.contains(&name.as_str());
                    let param = Self::json_schema_to_parameter(name, prop, is_required);
                    parameters.push(param);
                }
            }
        }

        parameters
    }

    /// Convert a JSON Schema property to a ToolParameter
    fn json_schema_to_parameter(name: &str, schema: &Value, required: bool) -> ToolParameter {
        let obj = schema.as_object();

        let param_type = obj
            .and_then(|o| o.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("string")
            .to_string();

        let description = obj
            .and_then(|o| o.get("description"))
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let default = obj.and_then(|o| o.get("default")).cloned();

        let enum_values = obj
            .and_then(|o| o.get("enum"))
            .and_then(|e| e.as_array())
            .cloned();

        let mut properties = HashMap::new();

        // Copy additional properties
        if let Some(o) = obj {
            for (key, value) in o {
                if !["type", "description", "default", "enum"].contains(&key.as_str()) {
                    properties.insert(key.clone(), value.clone());
                }
            }
        }

        ToolParameter {
            name: name.to_string(),
            description,
            param_type,
            required,
            default,
            enum_values,
            properties,
        }
    }

    /// Build a JSON Schema from ToolParameters
    pub fn parameters_to_json_schema(parameters: &[ToolParameter]) -> Value {
        let mut properties = Map::new();
        let mut required = Vec::new();

        for param in parameters {
            if param.required {
                required.push(param.name.clone());
            }

            let mut param_schema = Map::new();
            param_schema.insert("type".to_string(), json!(param.param_type));
            param_schema.insert("description".to_string(), json!(param.description));

            if let Some(default) = &param.default {
                param_schema.insert("default".to_string(), default.clone());
            }

            if let Some(enum_values) = &param.enum_values {
                param_schema.insert("enum".to_string(), json!(enum_values));
            }

            for (key, value) in &param.properties {
                param_schema.insert(key.clone(), value.clone());
            }

            properties.insert(param.name.clone(), Value::Object(param_schema));
        }

        json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }

    // ==========================================================================
    // Validation
    // ==========================================================================

    /// Validate that arguments match a schema
    pub fn validate_arguments(schema: &Value, arguments: &Value) -> Vec<String> {
        let mut errors = Vec::new();

        if let (Value::Object(schema_obj), Value::Object(args_obj)) = (schema, arguments) {
            // Check required fields
            if let Some(Value::Array(required)) = schema_obj.get("required") {
                for req in required {
                    if let Value::String(field_name) = req {
                        if !args_obj.contains_key(field_name) {
                            errors.push(format!("Missing required field: {}", field_name));
                        }
                    }
                }
            }

            // Check field types
            if let Some(Value::Object(properties)) = schema_obj.get("properties") {
                for (field_name, arg_value) in args_obj {
                    if let Some(prop_schema) = properties.get(field_name) {
                        if let Some(type_errors) = Self::validate_type(prop_schema, arg_value) {
                            for error in type_errors {
                                errors.push(format!("Field '{}': {}", field_name, error));
                            }
                        }
                    } else {
                        // Unknown field - warn but don't error
                        warn!("Unknown field in arguments: {}", field_name);
                    }
                }
            }
        }

        errors
    }

    /// Validate a value against a type schema
    fn validate_type(schema: &Value, value: &Value) -> Option<Vec<String>> {
        let mut errors = Vec::new();

        if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
            let actual_type = match value {
                Value::Null => "null",
                Value::Bool(_) => "boolean",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
            };

            // Handle type coercion for numbers
            let type_matches = match (expected_type, actual_type) {
                ("integer", "number") => value.as_f64().map(|n| n.fract() == 0.0).unwrap_or(false),
                (expected, actual) => expected == actual,
            };

            if !type_matches {
                errors.push(format!(
                    "Expected type '{}' but got '{}'",
                    expected_type, actual_type
                ));
            }
        }

        // Check enum values
        if let Some(Value::Array(enum_values)) = schema.get("enum") {
            if !enum_values.contains(value) {
                errors.push(format!("Value not in allowed enum: {:?}", enum_values));
            }
        }

        if errors.is_empty() {
            None
        } else {
            Some(errors)
        }
    }
}
