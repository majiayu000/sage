//! Tool format conversion for different providers

use crate::error::SageResult;
use crate::mcp::schema_translator::SchemaTranslator;
use crate::tools::types::ToolSchema;
use serde_json::{json, Value};

/// Tool format converter
pub struct ToolConverter;

impl ToolConverter {
    /// Convert tools for OpenAI format
    pub fn to_openai(tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for tool in tools {
            // Sanitize schema to ensure all description fields are valid strings
            let sanitized_params = SchemaTranslator::sanitize_json_schema(&tool.parameters);
            let tool_def = json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": sanitized_params
                }
            });
            converted.push(tool_def);
        }

        Ok(converted)
    }

    /// Convert tools for Anthropic format
    pub fn to_anthropic(tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for tool in tools {
            // Sanitize schema to ensure all description fields are valid strings
            // This prevents API errors like "description: Input should be a valid string"
            let sanitized_params = SchemaTranslator::sanitize_json_schema(&tool.parameters);
            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": sanitized_params
            });
            converted.push(tool_def);
        }

        Ok(converted)
    }

    /// Convert tools for GLM format (Anthropic-compatible but stricter)
    /// GLM doesn't accept empty "required": [] or empty "properties": {}
    pub fn to_glm(tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for tool in tools {
            // Sanitize schema to ensure all description fields are valid strings
            let mut schema = SchemaTranslator::sanitize_json_schema(&tool.parameters);

            // Remove empty "required" array
            if let Some(required) = schema.get("required") {
                if required.as_array().is_some_and(|arr| arr.is_empty()) {
                    schema.as_object_mut().map(|obj| obj.remove("required"));
                }
            }

            // Remove empty "properties" object
            if let Some(properties) = schema.get("properties") {
                if properties.as_object().is_some_and(|obj| obj.is_empty()) {
                    schema.as_object_mut().map(|obj| obj.remove("properties"));
                }
            }

            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": schema
            });
            converted.push(tool_def);
        }

        Ok(converted)
    }

    /// Convert tools for Google format
    pub fn to_google(tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for tool in tools {
            // Sanitize schema to ensure all description fields are valid strings
            let sanitized_params = SchemaTranslator::sanitize_json_schema(&tool.parameters);
            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": sanitized_params
            });
            converted.push(tool_def);
        }

        Ok(converted)
    }
}
