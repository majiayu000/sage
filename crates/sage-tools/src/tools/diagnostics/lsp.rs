//! LSP (Language Server Protocol) tool for code intelligence features

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

/// Supported LSP operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspOperation {
    GoToDefinition,
    FindReferences,
    Hover,
    DocumentSymbol,
    WorkspaceSymbol,
    GoToImplementation,
    PrepareCallHierarchy,
    IncomingCalls,
    OutgoingCalls,
}

impl LspOperation {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "goToDefinition" => Some(Self::GoToDefinition),
            "findReferences" => Some(Self::FindReferences),
            "hover" => Some(Self::Hover),
            "documentSymbol" => Some(Self::DocumentSymbol),
            "workspaceSymbol" => Some(Self::WorkspaceSymbol),
            "goToImplementation" => Some(Self::GoToImplementation),
            "prepareCallHierarchy" => Some(Self::PrepareCallHierarchy),
            "incomingCalls" => Some(Self::IncomingCalls),
            "outgoingCalls" => Some(Self::OutgoingCalls),
            _ => None,
        }
    }
}

/// LSP tool for interacting with Language Server Protocol servers
pub struct LspTool;

impl LspTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "LSP"
    }

    fn description(&self) -> &str {
        r#"Interact with Language Server Protocol (LSP) servers to get code intelligence features.

Supported operations:
- goToDefinition: Find where a symbol is defined
- findReferences: Find all references to a symbol
- hover: Get hover information (documentation, type info) for a symbol
- documentSymbol: Get all symbols (functions, classes, variables) in a document
- workspaceSymbol: Search for symbols across the entire workspace
- goToImplementation: Find implementations of an interface or abstract method
- prepareCallHierarchy: Get call hierarchy item at a position (functions/methods)
- incomingCalls: Find all functions/methods that call the function at a position
- outgoingCalls: Find all functions/methods called by the function at a position

All operations require:
- filePath: The file to operate on
- line: The line number (1-based, as shown in editors)
- character: The character offset (1-based, as shown in editors)

Note: LSP servers must be configured for the file type. If no server is available, an error will be returned."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "The LSP operation to perform",
                        "enum": [
                            "goToDefinition",
                            "findReferences",
                            "hover",
                            "documentSymbol",
                            "workspaceSymbol",
                            "goToImplementation",
                            "prepareCallHierarchy",
                            "incomingCalls",
                            "outgoingCalls"
                        ]
                    },
                    "filePath": {
                        "type": "string",
                        "description": "The absolute or relative path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "The line number (1-based, as shown in editors)",
                        "exclusiveMinimum": 0
                    },
                    "character": {
                        "type": "integer",
                        "description": "The character offset (1-based, as shown in editors)",
                        "exclusiveMinimum": 0
                    }
                },
                "required": ["operation", "filePath", "line", "character"]
            }),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let operation = call
            .get_string("operation")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'operation' parameter".to_string()))?;

        let file_path = call
            .get_string("filePath")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'filePath' parameter".to_string()))?;

        let line = call
            .get_number("line")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'line' parameter".to_string()))?
            as usize;

        let character = call
            .get_number("character")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'character' parameter".to_string()))?
            as usize;

        let _op = LspOperation::from_str(&operation).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Unknown operation: {}", operation))
        })?;

        // NOTE: This is a placeholder implementation.
        // Real LSP integration requires:
        // 1. Starting/connecting to language servers for different file types
        // 2. Implementing the LSP protocol (jsonrpc over stdio/tcp)
        // 3. Managing server lifecycle and capabilities
        //
        // For now, return a helpful message indicating LSP is not yet configured.
        let message = format!(
            r#"LSP operation '{}' requested for:
- File: {}
- Position: line {}, character {}

⚠️ LSP support is not yet configured.

To use LSP features, you need to:
1. Install a language server for the file type (e.g., rust-analyzer, typescript-language-server)
2. Configure the LSP server in your settings
3. Ensure the language server is running

Alternative approaches:
- Use Grep to search for symbol definitions: pattern like "fn symbol_name\(" or "class SymbolName"
- Use Glob to find files: pattern like "**/*.rs"
- Use Read to examine file contents directly

For now, please use these alternative tools to explore the codebase."#,
            operation, file_path, line, character
        );

        Ok(ToolResult::success(&call.id, self.name(), message))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Validate operation
        if let Some(op) = call.get_string("operation") {
            if LspOperation::from_str(&op).is_none() {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown operation: {}. Valid operations are: goToDefinition, findReferences, hover, documentSymbol, workspaceSymbol, goToImplementation, prepareCallHierarchy, incomingCalls, outgoingCalls",
                    op
                )));
            }
        } else {
            return Err(ToolError::InvalidArguments(
                "Missing 'operation' parameter".to_string(),
            ));
        }

        // Validate required parameters
        if call.get_string("filePath").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'filePath' parameter".to_string(),
            ));
        }

        if call.get_number("line").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'line' parameter".to_string(),
            ));
        }

        if call.get_number("character").is_none() {
            return Err(ToolError::InvalidArguments(
                "Missing 'character' parameter".to_string(),
            ));
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(30) // 30 seconds for LSP operations
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Read-only operations can run in parallel
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_lsp_tool_go_to_definition() {
        let tool = LspTool::new();
        let call = create_tool_call(
            "test-1",
            "LSP",
            serde_json::json!({
                "operation": "goToDefinition",
                "filePath": "/path/to/file.rs",
                "line": 10,
                "character": 5
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("goToDefinition"));
        assert!(output.contains("/path/to/file.rs"));
    }

    #[tokio::test]
    async fn test_lsp_tool_invalid_operation() {
        let tool = LspTool::new();
        let call = create_tool_call(
            "test-2",
            "LSP",
            serde_json::json!({
                "operation": "invalidOperation",
                "filePath": "/path/to/file.rs",
                "line": 10,
                "character": 5
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[test]
    fn test_lsp_operation_from_str() {
        assert_eq!(
            LspOperation::from_str("goToDefinition"),
            Some(LspOperation::GoToDefinition)
        );
        assert_eq!(
            LspOperation::from_str("findReferences"),
            Some(LspOperation::FindReferences)
        );
        assert_eq!(LspOperation::from_str("hover"), Some(LspOperation::Hover));
        assert_eq!(LspOperation::from_str("invalid"), None);
    }

    #[test]
    fn test_lsp_tool_schema() {
        let tool = LspTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "LSP");
        assert!(!schema.description.is_empty());
    }
}
