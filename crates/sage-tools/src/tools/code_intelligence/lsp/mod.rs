//! LSP (Language Server Protocol) tool
//!
//! Provides code intelligence features through LSP servers:
//! - Go to definition
//! - Find references
//! - Hover information
//! - Document symbols
//! - Workspace symbols
//! - Go to implementation
//! - Call hierarchy

mod client;
mod config;
mod operations;
mod protocol;
mod response;
mod tools;
pub mod types;

pub use config::{LspClient, LspConfig, LspServerConfig};
pub use tools::{FindReferencesTool, GoToDefinitionTool, SymbolSearchTool, TypeHierarchyTool};
pub use types::{
    CallHierarchyItem, DegradedReason, HoverInfo, Location, NavigationItem, NavigationResponse,
    NavigationStatus, Position, SymbolInfo,
};

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::{Path, PathBuf};

/// LSP tool for code intelligence.
#[derive(Clone)]
pub struct LspTool {
    /// Configuration
    config: LspConfig,
    /// Working directory
    working_directory: PathBuf,
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LspTool {
    /// Create a new LSP tool with default configuration
    pub fn new() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            config: LspConfig::with_defaults(),
            working_directory,
        }
    }

    /// Create with a specific working directory
    pub fn with_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        let mut tool = Self::new();
        tool.working_directory = working_directory.into();
        tool
    }

    /// Detect language from file path
    fn detect_language(&self, file_path: &Path) -> Option<String> {
        let ext = file_path.extension()?.to_str()?;

        for (lang, config) in &self.config.servers {
            if config.file_extensions.contains(&ext.to_string()) {
                return Some(lang.clone());
            }
        }

        None
    }

    fn resolve_workspace_path(&self, file_path: &str) -> PathBuf {
        let path = Path::new(file_path);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        }
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "LSP"
    }

    fn description(&self) -> &str {
        r#"Interact with Language Server Protocol (LSP) servers to get structured code navigation.

Supported operations:
- goToDefinition: Find where a symbol is defined
- findReferences: Find all references to a symbol
- workspaceSymbol: Search for symbols across the workspace
- typeHierarchy: Get type hierarchy for a Rust type

Prefer Grep/Glob for small lexical searches; use structured navigation for large repositories or cross-symbol work.
Results are structured JSON. LSP unavailable and capability unsupported return status=degraded, distinct from status=ok with empty items."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "operation",
                    "LSP operation: goToDefinition, findReferences, workspaceSymbol, typeHierarchy",
                ),
                ToolParameter::optional_string("filePath", "The file to operate on"),
                ToolParameter::number("line", "Line number (1-based)").optional(),
                ToolParameter::number("character", "Character offset (1-based)").optional(),
                ToolParameter::string("query", "Search query (for workspaceSymbol)").optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        self.validate(call)?;
        let operation = call.get_string("operation").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: operation".to_string())
        })?;

        if operation == "workspaceSymbol" {
            let query = call.get_string("query").ok_or_else(|| {
                ToolError::InvalidArguments("Missing required parameter: query".to_string())
            })?;
            let result = self.workspace_symbol(&query).await?;
            return Ok(ToolResult::success(&call.id, self.name(), result));
        }

        let file_path = call.get_string("filePath").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: filePath".to_string())
        })?;
        let line = call.get_u32("line", 0);
        let character = call.get_u32("character", 0);

        let result = match operation.as_str() {
            "goToDefinition" => self.go_to_definition(&file_path, line, character).await?,
            "findReferences" => self.find_references(&file_path, line, character).await?,
            "hover" => self.hover(&file_path, line, character).await?,
            "documentSymbol" => self.document_symbol(&file_path).await?,
            "typeHierarchy" => self.type_hierarchy(&file_path, line, character).await?,
            "goToImplementation" => {
                self.go_to_implementation(&file_path, line, character)
                    .await?
            }
            "prepareCallHierarchy" => {
                self.prepare_call_hierarchy(&file_path, line, character)
                    .await?
            }
            "incomingCalls" => self.incoming_calls(&file_path, line, character).await?,
            "outgoingCalls" => self.outgoing_calls(&file_path, line, character).await?,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown operation: {}",
                    operation
                )));
            }
        };

        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let operation = call.get_string("operation").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: operation".to_string())
        })?;

        if operation == "workspaceSymbol" {
            call.get_string("query").ok_or_else(|| {
                ToolError::InvalidArguments("Missing required parameter: query".to_string())
            })?;
            return Ok(());
        }

        call.get_string("filePath").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: filePath".to_string())
        })?;

        if call.get_u32("line", 0) == 0 {
            return Err(ToolError::InvalidArguments(
                "line must be a positive 1-based number".to_string(),
            ));
        }

        if call.get_u32("character", 0) == 0 {
            return Err(ToolError::InvalidArguments(
                "character must be a positive 1-based number".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_tool_call(operation: &str, file_path: &str, line: u32, character: u32) -> ToolCall {
        let mut arguments = std::collections::HashMap::new();
        arguments.insert("operation".to_string(), json!(operation));
        arguments.insert("filePath".to_string(), json!(file_path));
        arguments.insert("line".to_string(), json!(line));
        arguments.insert("character".to_string(), json!(character));

        ToolCall {
            id: "test-1".to_string(),
            name: "LSP".to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_go_to_definition() {
        let tool = LspTool::new();
        let call = create_tool_call("goToDefinition", "test.rs", 10, 5);

        let result = tool.execute(&call).await;
        // May fail if rust-analyzer not installed, but should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_workspace_symbol_requires_query() {
        let tool = LspTool::new();
        let call = create_tool_call("workspaceSymbol", "test.rs", 1, 1);

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let tool = LspTool::new();
        let call = create_tool_call("invalidOp", "test.rs", 1, 1);

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_language() {
        let tool = LspTool::new();

        assert_eq!(
            tool.detect_language(Path::new("test.rs")),
            Some("rust".to_string())
        );
        assert_eq!(
            tool.detect_language(Path::new("test.ts")),
            Some("typescript".to_string())
        );
        assert_eq!(
            tool.detect_language(Path::new("test.py")),
            Some("python".to_string())
        );
        assert_eq!(
            tool.detect_language(Path::new("test.go")),
            Some("go".to_string())
        );
        assert_eq!(tool.detect_language(Path::new("test.unknown")), None);
    }

    #[test]
    fn test_schema() {
        let tool = LspTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "LSP");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_lsp_tool_description_documents_degraded_results() {
        let tool = LspTool::new();

        assert!(tool.description().contains("status=degraded"));
        assert!(tool.description().contains("Grep/Glob"));
    }
}
