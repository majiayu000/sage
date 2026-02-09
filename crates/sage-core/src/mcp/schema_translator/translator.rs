//! Core translation logic between Sage and MCP schemas

use crate::mcp::types::{McpContent, McpTool, McpToolResult};
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Translator for converting between Sage and MCP schema formats
pub struct SchemaTranslator;

impl SchemaTranslator {
    // ==========================================================================
    // Tool Schema Translation (MCP -> Sage)
    // ==========================================================================

    /// Convert an MCP tool to a Sage ToolSchema
    /// Sanitizes the input_schema to ensure all description fields are valid strings
    pub fn mcp_to_sage_schema(mcp_tool: &McpTool) -> ToolSchema {
        let sanitized_schema = Self::sanitize_json_schema(&mcp_tool.input_schema);
        ToolSchema {
            name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone().unwrap_or_default(),
            parameters: sanitized_schema,
        }
    }

    /// Sanitize a JSON schema to ensure all description fields are valid strings
    /// This prevents API errors like "description: Input should be a valid string"
    pub fn sanitize_json_schema(schema: &Value) -> Value {
        match schema {
            Value::Object(obj) => {
                let mut new_obj = Map::new();
                for (key, value) in obj {
                    if key == "description" {
                        // Ensure description is always a string
                        let desc_str = match value {
                            Value::String(s) => s.clone(),
                            Value::Null => String::new(),
                            other => other.to_string(),
                        };
                        new_obj.insert(key.clone(), Value::String(desc_str));
                    } else if key == "properties" {
                        // Recursively sanitize properties
                        if let Value::Object(props) = value {
                            let mut new_props = Map::new();
                            for (prop_name, prop_schema) in props {
                                new_props.insert(
                                    prop_name.clone(),
                                    Self::sanitize_json_schema(prop_schema),
                                );
                            }
                            new_obj.insert(key.clone(), Value::Object(new_props));
                        } else {
                            new_obj.insert(key.clone(), value.clone());
                        }
                    } else if key == "items" {
                        // Recursively sanitize array items schema
                        new_obj.insert(key.clone(), Self::sanitize_json_schema(value));
                    } else if key == "additionalProperties" && value.is_object() {
                        // Recursively sanitize additionalProperties if it's a schema
                        new_obj.insert(key.clone(), Self::sanitize_json_schema(value));
                    } else if key == "anyOf" || key == "oneOf" || key == "allOf" {
                        // Recursively sanitize schema arrays
                        if let Value::Array(arr) = value {
                            let sanitized: Vec<Value> =
                                arr.iter().map(|v| Self::sanitize_json_schema(v)).collect();
                            new_obj.insert(key.clone(), Value::Array(sanitized));
                        } else {
                            new_obj.insert(key.clone(), value.clone());
                        }
                    } else {
                        new_obj.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(new_obj)
            }
            other => other.clone(),
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
}
