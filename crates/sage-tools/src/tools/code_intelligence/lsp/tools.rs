//! Agent-facing structured code navigation tools.

use super::LspTool;
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;

macro_rules! position_tool {
    ($tool:ident, $name:literal, $description:literal, $method:ident) => {
        #[derive(Clone)]
        pub struct $tool {
            lsp: LspTool,
        }

        impl Default for $tool {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $tool {
            pub fn new() -> Self {
                Self {
                    lsp: LspTool::new(),
                }
            }

            pub fn with_working_directory(working_directory: impl Into<PathBuf>) -> Self {
                Self {
                    lsp: LspTool::with_working_directory(working_directory),
                }
            }
        }

        #[async_trait]
        impl Tool for $tool {
            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $description
            }

            fn schema(&self) -> ToolSchema {
                ToolSchema::new(
                    self.name(),
                    self.description(),
                    vec![
                        ToolParameter::string(
                            "file_path",
                            "Absolute or workspace-relative source file path.",
                        ),
                        ToolParameter::number("line", "1-based line number."),
                        ToolParameter::number("character", "1-based character offset."),
                    ],
                )
            }

            async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
                let file_path = require_file_path(call)?;
                let line = require_positive_u32(call, "line")?;
                let character = require_positive_u32(call, "character")?;
                let output = self.lsp.$method(&file_path, line, character).await?;
                Ok(ToolResult::success(&call.id, self.name(), output))
            }

            fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
                require_file_path(call)?;
                require_positive_u32(call, "line")?;
                require_positive_u32(call, "character")?;
                Ok(())
            }

            fn max_execution_duration(&self) -> Option<std::time::Duration> {
                Some(std::time::Duration::from_secs(30))
            }

            fn supports_parallel_execution(&self) -> bool {
                true
            }

            fn is_read_only(&self) -> bool {
                true
            }
        }
    };
}

position_tool!(
    GoToDefinitionTool,
    "GoToDefinition",
    "Use LSP to locate the definition for a symbol at a position. Prefer Grep/Glob for small lexical searches; use this for large repositories or cross-symbol navigation. Returns structured ok/degraded JSON and never silently falls back to grep.",
    go_to_definition
);

position_tool!(
    FindReferencesTool,
    "FindReferences",
    "Use LSP to find references for a symbol at a position. Prefer Grep/Glob for small lexical searches; use this for large repositories or cross-symbol navigation. Returns structured ok/degraded JSON and never silently falls back to grep.",
    find_references
);

position_tool!(
    TypeHierarchyTool,
    "TypeHierarchy",
    "Use LSP type hierarchy for a Rust type at a position. Prefer Grep/Glob for small lexical searches; use this for large repositories or type relationships. Returns structured ok/degraded JSON and never silently falls back to grep.",
    type_hierarchy
);

#[derive(Clone)]
pub struct SymbolSearchTool {
    lsp: LspTool,
}

impl Default for SymbolSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolSearchTool {
    pub fn new() -> Self {
        Self {
            lsp: LspTool::new(),
        }
    }

    pub fn with_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            lsp: LspTool::with_working_directory(working_directory),
        }
    }
}

#[async_trait]
impl Tool for SymbolSearchTool {
    fn name(&self) -> &str {
        "SymbolSearch"
    }

    fn description(&self) -> &str {
        "Use LSP workspace/symbol search. Prefer Grep/Glob for small lexical searches; use this for large repositories or cross-symbol navigation. Defaults to Rust and returns structured ok/degraded JSON without grep fallback."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("query", "Symbol query."),
                ToolParameter::optional_string(
                    "language",
                    "Language server to query. Defaults to rust.",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let query = call
            .get_string("query")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".to_string()))?;
        let language = call
            .get_string("language")
            .unwrap_or_else(|| "rust".to_string());
        let output = self
            .lsp
            .workspace_symbol_for_language(&query, &language)
            .await?;
        Ok(ToolResult::success(&call.id, self.name(), output))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        call.get_string("query")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".to_string()))?;
        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(30))
    }

    fn supports_parallel_execution(&self) -> bool {
        true
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

fn require_file_path(call: &ToolCall) -> Result<String, ToolError> {
    call.get_string("file_path")
        .ok_or_else(|| ToolError::InvalidArguments("Missing 'file_path' parameter".to_string()))
}

fn require_positive_u32(call: &ToolCall, name: &str) -> Result<u32, ToolError> {
    let value = call.get_u32(name, 0);
    if value == 0 {
        return Err(ToolError::InvalidArguments(format!(
            "'{}' must be a positive 1-based number",
            name
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn call(name: &str, arguments: serde_json::Value) -> ToolCall {
        let arguments = arguments.as_object().unwrap().clone();
        ToolCall {
            id: "call-1".to_string(),
            name: name.to_string(),
            arguments: arguments.into_iter().collect::<HashMap<_, _>>(),
            call_id: None,
        }
    }

    #[tokio::test]
    async fn dedicated_tool_reports_degraded_when_lsp_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("lib.rs");
        tokio::fs::write(&source_path, "pub fn sample() {}\n")
            .await
            .unwrap();

        let mut lsp = LspTool::with_working_directory(dir.path());
        lsp.config.servers.get_mut("rust").unwrap().command =
            "sage-rust-analyzer-missing-for-test".to_string();
        let tool = GoToDefinitionTool { lsp };

        let result = tool
            .execute(&call(
                "GoToDefinition",
                json!({
                    "file_path": "lib.rs",
                    "line": 1,
                    "character": 8
                }),
            ))
            .await
            .unwrap();

        let output: serde_json::Value =
            serde_json::from_str(result.output.as_deref().unwrap()).unwrap();
        assert_eq!(output["status"], "degraded");
        assert_eq!(output["reason"], "lsp_unavailable");
    }

    #[test]
    fn position_tool_schema_documents_structured_navigation_strategy() {
        let schema = FindReferencesTool::new().schema();

        assert_eq!(schema.name, "FindReferences");
        assert!(schema.description.contains("Grep/Glob"));
        assert!(schema.description.contains("degraded"));
    }
}
