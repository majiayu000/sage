//! Tool schema and trait implementation for codebase retrieval

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

use super::CodebaseRetrievalTool;

#[async_trait]
impl Tool for CodebaseRetrievalTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Enhanced codebase search engine that finds relevant code snippets using intelligent pattern matching. Supports function names, class names, keywords, and file patterns across multiple programming languages."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let information_request = tool_call
            .arguments
            .get("information_request")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments(
                    "Missing required parameter: information_request".to_string(),
                )
            })?;

        match self.search_codebase(information_request).await {
            Ok(result) => Ok(ToolResult::success(&tool_call.id, self.name(), result)),
            Err(e) => Ok(ToolResult::error(
                &tool_call.id,
                self.name(),
                format!("Codebase retrieval failed: {}", e),
            )),
        }
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "information_request": {
                        "type": "string",
                        "description": "Description of what you're looking for in the codebase. Can include function names, class names, keywords, or file patterns."
                    }
                },
                "required": ["information_request"]
            }),
        }
    }
}
