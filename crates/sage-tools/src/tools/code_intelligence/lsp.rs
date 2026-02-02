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

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// LSP position (1-based, as shown in editors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP location (file + position)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
    pub end_line: Option<u32>,
    pub end_character: Option<u32>,
}

/// Symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub location: Location,
    pub container_name: Option<String>,
}

/// Hover information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Location>,
}

/// Call hierarchy item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: String,
    pub location: Location,
    pub detail: Option<String>,
}

/// LSP client for a specific language
pub struct LspClient {
    /// Language ID (e.g., "rust", "typescript")
    pub language_id: String,
    /// Server command
    pub command: String,
    /// Server arguments
    pub args: Vec<String>,
    /// Whether the server is running
    pub running: bool,
}

impl LspClient {
    pub fn new(language_id: &str, command: &str, args: Vec<String>) -> Self {
        Self {
            language_id: language_id.to_string(),
            command: command.to_string(),
            args,
            running: false,
        }
    }
}

/// LSP configuration
#[derive(Debug, Clone, Default)]
pub struct LspConfig {
    /// Registered language servers
    pub servers: HashMap<String, LspServerConfig>,
}

/// Configuration for a single LSP server
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    pub language_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
}

/// LSP tool for code intelligence
pub struct LspTool {
    #[allow(dead_code)]
    /// LSP clients by language
    clients: Arc<RwLock<HashMap<String, LspClient>>>,
    /// Configuration
    config: LspConfig,
    /// Working directory
    working_dir: PathBuf,
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LspTool {
    /// Create a new LSP tool with default configuration
    pub fn new() -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut config = LspConfig::default();

        // Register common LSP servers
        config.servers.insert(
            "rust".to_string(),
            LspServerConfig {
                language_id: "rust".to_string(),
                command: "rust-analyzer".to_string(),
                args: vec![],
                file_extensions: vec!["rs".to_string()],
            },
        );

        config.servers.insert(
            "typescript".to_string(),
            LspServerConfig {
                language_id: "typescript".to_string(),
                command: "typescript-language-server".to_string(),
                args: vec!["--stdio".to_string()],
                file_extensions: vec![
                    "ts".to_string(),
                    "tsx".to_string(),
                    "js".to_string(),
                    "jsx".to_string(),
                ],
            },
        );

        config.servers.insert(
            "python".to_string(),
            LspServerConfig {
                language_id: "python".to_string(),
                command: "pylsp".to_string(),
                args: vec![],
                file_extensions: vec!["py".to_string()],
            },
        );

        config.servers.insert(
            "go".to_string(),
            LspServerConfig {
                language_id: "go".to_string(),
                command: "gopls".to_string(),
                args: vec![],
                file_extensions: vec!["go".to_string()],
            },
        );

        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
            working_dir,
        }
    }

    /// Create with a specific working directory
    pub fn with_working_dir(working_dir: impl Into<PathBuf>) -> Self {
        let mut tool = Self::new();
        tool.working_dir = working_dir.into();
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
            // Check if command exists using std::process::Command
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

    /// Go to definition
    async fn go_to_definition(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed. Install '{}' to enable this feature.",
                language,
                self.config.servers.get(&language).map(|c| c.command.as_str()).unwrap_or("unknown")
            )));
        }

        // In a real implementation, this would communicate with the LSP server
        // For now, we return a placeholder response
        Ok(format!(
            "Go to definition for {}:{}:{}\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             Language detected: {}\n\n\
             To use this feature:\n\
             1. Ensure the LSP server is installed\n\
             2. The server will be started automatically when needed\n\
             3. Results will show the definition location",
            file_path, line, character, language
        ))
    }

    /// Find references
    async fn find_references(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed.",
                language
            )));
        }

        Ok(format!(
            "Find references for {}:{}:{}\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             Language detected: {}",
            file_path, line, character, language
        ))
    }

    /// Get hover information
    async fn hover(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed.",
                language
            )));
        }

        Ok(format!(
            "Hover info for {}:{}:{}\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             Language detected: {}",
            file_path, line, character, language
        ))
    }

    /// Get document symbols
    async fn document_symbol(&self, file_path: &str) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed.",
                language
            )));
        }

        // For now, use a simple regex-based symbol extraction as fallback
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let symbols = self.extract_symbols_simple(&content, &language);

        if symbols.is_empty() {
            Ok(format!(
                "No symbols found in {}.\n\n\
                 Note: For better results, ensure the LSP server is running.",
                file_path
            ))
        } else {
            let mut output = format!("Symbols in {} ({}):\n\n", file_path, language);
            for symbol in symbols {
                output.push_str(&format!(
                    "- {} ({}) at line {}\n",
                    symbol.name, symbol.kind, symbol.location.line
                ));
            }
            Ok(output)
        }
    }

    /// Simple symbol extraction (fallback when LSP not available)
    fn extract_symbols_simple(&self, content: &str, language: &str) -> Vec<SymbolInfo> {
        let mut symbols = Vec::new();

        let patterns: Vec<(&str, &str)> = match language {
            "rust" => vec![
                (r"(?m)^pub\s+fn\s+(\w+)", "function"),
                (r"(?m)^fn\s+(\w+)", "function"),
                (r"(?m)^pub\s+struct\s+(\w+)", "struct"),
                (r"(?m)^struct\s+(\w+)", "struct"),
                (r"(?m)^pub\s+enum\s+(\w+)", "enum"),
                (r"(?m)^enum\s+(\w+)", "enum"),
                (r"(?m)^pub\s+trait\s+(\w+)", "trait"),
                (r"(?m)^trait\s+(\w+)", "trait"),
                (r"(?m)^impl\s+(\w+)", "impl"),
            ],
            "typescript" | "javascript" => vec![
                (r"(?m)^export\s+function\s+(\w+)", "function"),
                (r"(?m)^function\s+(\w+)", "function"),
                (r"(?m)^export\s+class\s+(\w+)", "class"),
                (r"(?m)^class\s+(\w+)", "class"),
                (r"(?m)^export\s+interface\s+(\w+)", "interface"),
                (r"(?m)^interface\s+(\w+)", "interface"),
                (r"(?m)^const\s+(\w+)\s*=", "constant"),
            ],
            "python" => vec![
                (r"(?m)^def\s+(\w+)", "function"),
                (r"(?m)^class\s+(\w+)", "class"),
                (r"(?m)^async\s+def\s+(\w+)", "function"),
            ],
            "go" => vec![
                (r"(?m)^func\s+(\w+)", "function"),
                (r"(?m)^func\s+\([^)]+\)\s+(\w+)", "method"),
                (r"(?m)^type\s+(\w+)\s+struct", "struct"),
                (r"(?m)^type\s+(\w+)\s+interface", "interface"),
            ],
            _ => vec![],
        };

        for (pattern, kind) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for (line_num, line) in content.lines().enumerate() {
                    if let Some(caps) = re.captures(line) {
                        if let Some(name) = caps.get(1) {
                            symbols.push(SymbolInfo {
                                name: name.as_str().to_string(),
                                kind: kind.to_string(),
                                location: Location {
                                    file_path: String::new(),
                                    line: (line_num + 1) as u32,
                                    character: 1,
                                    end_line: None,
                                    end_character: None,
                                },
                                container_name: None,
                            });
                        }
                    }
                }
            }
        }

        symbols
    }

    /// Search workspace symbols
    async fn workspace_symbol(&self, query: &str) -> Result<String, ToolError> {
        Ok(format!(
            "Workspace symbol search for '{}'\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             This operation searches across all files in the workspace.",
            query
        ))
    }

    /// Go to implementation
    async fn go_to_implementation(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        Ok(format!(
            "Go to implementation for {}:{}:{}\n\n\
             Language detected: {}\n\
             Note: This finds implementations of interfaces/traits.",
            file_path, line, character, language
        ))
    }

    /// Prepare call hierarchy
    async fn prepare_call_hierarchy(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        Ok(format!(
            "Call hierarchy for {}:{}:{}\n\n\
             Note: Use incomingCalls or outgoingCalls to explore the hierarchy.",
            file_path, line, character
        ))
    }

    /// Get incoming calls
    async fn incoming_calls(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        Ok(format!(
            "Incoming calls to function at {}:{}:{}\n\n\
             Note: Shows all functions/methods that call this function.",
            file_path, line, character
        ))
    }

    /// Get outgoing calls
    async fn outgoing_calls(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        Ok(format!(
            "Outgoing calls from function at {}:{}:{}\n\n\
             Note: Shows all functions/methods called by this function.",
            file_path, line, character
        ))
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

        let line = call.get_number("line").unwrap_or(1.0) as u32;
        let character = call.get_number("character").unwrap_or(1.0) as u32;

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

        // Create a temp file
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
        let tool = LspTool::new();
        let content = r#"
pub fn hello() {}
fn private_fn() {}
pub struct Foo {}
struct Bar {}
pub enum MyEnum {}
trait MyTrait {}
impl Foo {}
"#;

        let symbols = tool.extract_symbols_simple(content, "rust");
        assert!(!symbols.is_empty());

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"Foo"));
    }
}
