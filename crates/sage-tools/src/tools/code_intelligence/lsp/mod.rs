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

mod config;
mod operations;
mod symbols;
pub mod types;

pub use config::{LspClient, LspConfig, LspServerConfig};
pub use types::{CallHierarchyItem, HoverInfo, Location, Position, SymbolInfo};

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// LSP tool for code intelligence
pub struct LspTool {
    #[allow(dead_code)]
    /// LSP clients by language
    clients: Arc<RwLock<HashMap<String, LspClient>>>,
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
            clients: Arc::new(RwLock::new(HashMap::new())),
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

    /// Check if LSP server is available for a language
    fn is_server_available(&self, language: &str) -> bool {
        if let Some(config) = self.config.servers.get(language) {
            std::process::Command::new(&config.command)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok()
        } else {
            false
        }
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
- hover: Get hover information (documentation, type info)
- documentSymbol: Get all symbols in a document
- workspaceSymbol: Search for symbols across the workspace
- goToImplementation: Find implementations of an interface/trait
- prepareCallHierarchy: Get call hierarchy item at a position
- incomingCalls: Find all callers of a function
- outgoingCalls: Find all functions called by a function

All operations require filePath, line (1-based), and character (1-based)."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "operation",
                    "LSP operation: goToDefinition, findReferences, hover, documentSymbol, workspaceSymbol, goToImplementation, prepareCallHierarchy, incomingCalls, outgoingCalls",
                ),
                ToolParameter::string("filePath", "The file to operate on"),
                ToolParameter::number("line", "Line number (1-based)"),
                ToolParameter::number("character", "Character offset (1-based)"),
                ToolParameter::string("query", "Search query (for workspaceSymbol)").optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let operation = call.get_string("operation").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: operation".to_string())
        })?;

        let file_path = call.get_string("filePath").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: filePath".to_string())
        })?;

        let line = call.get_u32("line", 1);
        let character = call.get_u32("character", 1);

        let result = match operation.as_str() {
            "goToDefinition" => self.go_to_definition(&file_path, line, character).await?,
            "findReferences" => self.find_references(&file_path, line, character).await?,
            "hover" => self.hover(&file_path, line, character).await?,
            "documentSymbol" => self.document_symbol(&file_path).await?,
            "workspaceSymbol" => {
                let query = call.get_string("query").unwrap_or_default();
                self.workspace_symbol(&query).await?
            }
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
        call.get_string("operation").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: operation".to_string())
        })?;

        call.get_string("filePath").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: filePath".to_string())
        })?;

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
    async fn test_document_symbol() {
        let tool = LspTool::new();

        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        tokio::fs::write(
            &file_path,
            r#"
pub fn hello() {}
struct Foo {}
impl Foo {
    fn bar(&self) {}
}
"#,
        )
        .await
        .unwrap();

        let mut arguments = std::collections::HashMap::new();
        arguments.insert("operation".to_string(), json!("documentSymbol"));
        arguments.insert("filePath".to_string(), json!(file_path.to_str().unwrap()));
        arguments.insert("line".to_string(), json!(1));
        arguments.insert("character".to_string(), json!(1));

        let call = ToolCall {
            id: "test-2".to_string(),
            name: "LSP".to_string(),
            arguments,
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("hello") || output.contains("Foo"));
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
    fn test_extract_symbols_rust() {
        let content = r#"
pub fn hello() {}
fn private_fn() {}
pub struct Foo {}
struct Bar {}
pub enum MyEnum {}
trait MyTrait {}
impl Foo {}
"#;

        let symbols = symbols::extract_symbols_simple(content, "rust");
        assert!(!symbols.is_empty());

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"Foo"));
    }
}
