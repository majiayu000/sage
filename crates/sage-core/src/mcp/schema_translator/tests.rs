//! Tests for schema translation functionality

#[cfg(test)]
mod tests {
    use super::super::translator::SchemaTranslator;
    use crate::mcp::types::{McpContent, McpTool, McpToolResult};
    use crate::tools::types::{ToolCall, ToolParameter, ToolSchema};
    use serde_json::json;
    use std::collections::HashMap;

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
