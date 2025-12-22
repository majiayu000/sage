//! Tool format conversion for different providers

use crate::error::SageResult;
use crate::tools::types::ToolSchema;
use serde_json::{json, Value};

/// Tool format converter
pub struct ToolConverter;

impl ToolConverter {
    /// Convert tools for OpenAI format
    pub fn to_openai(tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for tool in tools {
            let tool_def = json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
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
            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters
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
            // Clone the parameters and clean up empty arrays/objects
            let mut schema = tool.parameters.clone();

            // Remove empty "required" array
            if let Some(required) = schema.get("required") {
                if required.as_array().map_or(false, |arr| arr.is_empty()) {
                    schema.as_object_mut().map(|obj| obj.remove("required"));
                }
            }

            // Remove empty "properties" object
            if let Some(properties) = schema.get("properties") {
                if properties.as_object().map_or(false, |obj| obj.is_empty()) {
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
            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters
            });
            converted.push(tool_def);
        }

        Ok(converted)
    }
}
