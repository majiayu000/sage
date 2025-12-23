//! MCP Schema Translator
//!
//! Provides bidirectional translation between Sage tool schemas and MCP tool schemas.
//! This enables interoperability between Sage's internal tool system and MCP servers.

use super::types::{McpContent, McpTool, McpToolResult};
use crate::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use tracing::warn;

/// Translator for converting between Sage and MCP schema formats
pub struct SchemaTranslator;

impl SchemaTranslator {
    // ==========================================================================
    // Tool Schema Translation (MCP -> Sage)
    // ==========================================================================

    /// Convert an MCP tool to a Sage ToolSchema
    pub fn mcp_to_sage_schema(mcp_tool: &McpTool) -> ToolSchema {
        ToolSchema {
            name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone().unwrap_or_default(),
            parameters: mcp_tool.input_schema.clone(),
        }
    }

    /// Convert multiple MCP tools to Sage ToolSchemas
    pub fn mcp_tools_to_sage_schemas(mcp_tools: &[McpTool]) -> Vec<ToolSchema> {
        mcp_tools.iter().map(Self::mcp_to_sage_schema).collect()
    }

    // ==========================================================================
    // Tool Schema Translation (Sage -> MCP)
    // ==========================================================================

    /// Convert a Sage ToolSchema to an MCP tool
    pub fn sage_to_mcp_tool(schema: &ToolSchema) -> McpTool {
        McpTool {
            name: schema.name.clone(),
            description: Some(schema.description.clone()),
            input_schema: schema.parameters.clone(),
        }
    }

    /// Convert multiple Sage ToolSchemas to MCP tools
    pub fn sage_schemas_to_mcp_tools(schemas: &[ToolSchema]) -> Vec<McpTool> {
        schemas.iter().map(Self::sage_to_mcp_tool).collect()
    }

    // ==========================================================================
    // Tool Call Translation
    // ==========================================================================

    /// Convert a Sage ToolCall to MCP format (tool name + arguments)
    pub fn sage_call_to_mcp(call: &ToolCall) -> (String, Value) {
        let args: Value = call
            .arguments
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        (call.name.clone(), args)
    }

    /// Convert MCP tool call parameters to a Sage ToolCall
    pub fn mcp_to_sage_call(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        arguments: Value,
    ) -> ToolCall {
        let args_map: HashMap<String, Value> = match arguments {
            Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };

        ToolCall {
            id: call_id.into(),
            name: tool_name.into(),
            arguments: args_map,
            call_id: None,
        }
    }

    // ==========================================================================
    // Tool Result Translation
    // ==========================================================================

    /// Convert an MCP tool result to a Sage ToolResult
    pub fn mcp_result_to_sage(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        mcp_result: &McpToolResult,
    ) -> ToolResult {
        let text = mcp_result
            .content
            .iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        if mcp_result.is_error {
            ToolResult::error(call_id, tool_name, text)
        } else {
            ToolResult::success(call_id, tool_name, text)
        }
    }

    /// Convert a Sage ToolResult to MCP format
    pub fn sage_result_to_mcp(result: &ToolResult) -> McpToolResult {
        let text = result
            .output
            .clone()
            .or_else(|| result.error.clone())
            .unwrap_or_default();

        McpToolResult {
            content: vec![McpContent::Text { text }],
            is_error: !result.success,
        }
    }

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
    // Content Type Translation
    // ==========================================================================

    /// Convert MCP content array to a single string
    pub fn mcp_content_to_string(content: &[McpContent]) -> String {
        content
            .iter()
            .map(|c| match c {
                McpContent::Text { text } => text.clone(),
                McpContent::Image { data, mime_type } => {
                    format!("[Image: {} ({} bytes)]", mime_type, data.len())
                }
                McpContent::Resource { resource } => format!("[Resource: {}]", resource.uri),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert a string to MCP text content
    pub fn string_to_mcp_content(text: impl Into<String>) -> Vec<McpContent> {
        vec![McpContent::Text { text: text.into() }]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_to_sage_schema() {
        let mcp_tool = McpTool {
            name: "test_tool".to_string(),
            description: Some("Test description".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string" }
                }
            }),
        };

        let sage_schema = SchemaTranslator::mcp_to_sage_schema(&mcp_tool);

        assert_eq!(sage_schema.name, "test_tool");
        assert_eq!(sage_schema.description, "Test description");
    }

    #[test]
    fn test_sage_to_mcp_tool() {
        let sage_schema = ToolSchema {
            name: "sage_tool".to_string(),
            description: "Sage description".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        };

        let mcp_tool = SchemaTranslator::sage_to_mcp_tool(&sage_schema);

        assert_eq!(mcp_tool.name, "sage_tool");
        assert_eq!(mcp_tool.description, Some("Sage description".to_string()));
    }

    #[test]
    fn test_sage_call_to_mcp() {
        let mut args = HashMap::new();
        args.insert("path".to_string(), json!("/tmp/test"));
        args.insert("content".to_string(), json!("hello"));

        let call = ToolCall {
            id: "1".to_string(),
            name: "write_file".to_string(),
            arguments: args,
            call_id: None,
        };

        let (name, mcp_args) = SchemaTranslator::sage_call_to_mcp(&call);

        assert_eq!(name, "write_file");
        assert_eq!(mcp_args["path"], json!("/tmp/test"));
        assert_eq!(mcp_args["content"], json!("hello"));
    }

    #[test]
    fn test_mcp_to_sage_call() {
        let args = json!({
            "filename": "test.txt",
            "data": "content"
        });

        let call = SchemaTranslator::mcp_to_sage_call("call-1", "read_file", args);

        assert_eq!(call.id, "call-1");
        assert_eq!(call.name, "read_file");
        assert_eq!(call.arguments.get("filename"), Some(&json!("test.txt")));
    }

    #[test]
    fn test_mcp_result_to_sage_success() {
        let mcp_result = McpToolResult {
            content: vec![McpContent::Text {
                text: "Success!".to_string(),
            }],
            is_error: false,
        };

        let sage_result = SchemaTranslator::mcp_result_to_sage("call-1", "test_tool", &mcp_result);

        assert!(sage_result.success);
        assert_eq!(sage_result.output, Some("Success!".to_string()));
    }

    #[test]
    fn test_mcp_result_to_sage_error() {
        let mcp_result = McpToolResult {
            content: vec![McpContent::Text {
                text: "Error occurred".to_string(),
            }],
            is_error: true,
        };

        let sage_result = SchemaTranslator::mcp_result_to_sage("call-1", "test_tool", &mcp_result);

        assert!(!sage_result.success);
        assert_eq!(sage_result.error, Some("Error occurred".to_string()));
    }

    #[test]
    fn test_extract_parameters_from_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "The name" },
                "count": { "type": "integer", "description": "The count" }
            },
            "required": ["name"]
        });

        let params = SchemaTranslator::extract_parameters_from_schema(&schema);

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "name" && p.required));
        assert!(params.iter().any(|p| p.name == "count" && !p.required));
    }

    #[test]
    fn test_parameters_to_json_schema() {
        let params = vec![
            ToolParameter::string("name", "The name"),
            ToolParameter::number("count", "The count").optional(),
        ];

        let schema = SchemaTranslator::parameters_to_json_schema(&params);

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"]["type"] == "string");
        assert!(schema["properties"]["count"]["type"] == "number");
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("name"))
        );
    }

    #[test]
    fn test_validate_arguments_valid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number" }
            },
            "required": ["name"]
        });

        let args = json!({
            "name": "John",
            "age": 30
        });

        let errors = SchemaTranslator::validate_arguments(&schema, &args);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_arguments_missing_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });

        let args = json!({});

        let errors = SchemaTranslator::validate_arguments(&schema, &args);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Missing required field"));
    }

    #[test]
    fn test_validate_arguments_wrong_type() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": { "type": "number" }
            }
        });

        let args = json!({
            "count": "not a number"
        });

        let errors = SchemaTranslator::validate_arguments(&schema, &args);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Expected type"));
    }

    #[test]
    fn test_mcp_content_to_string() {
        let content = vec![
            McpContent::Text {
                text: "Line 1".to_string(),
            },
            McpContent::Text {
                text: "Line 2".to_string(),
            },
        ];

        let result = SchemaTranslator::mcp_content_to_string(&content);
        assert_eq!(result, "Line 1\nLine 2");
    }
}
