use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct WebFetchTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebFetchInput {
    pub url: String,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web-fetch"
    }

    fn description(&self) -> &str {
        "Fetches data from a webpage and converts it into Markdown.\n\n1. The tool takes in a URL and returns the content of the page in Markdown format;\n2. If the return is not valid Markdown, it means the tool cannot successfully parse this page."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string("url", "The URL to fetch.")],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let url = call
            .get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        // TODO: Implement actual web fetching functionality
        // This is a placeholder implementation
        let markdown_content = format!(
            "# Fetched Content from {}\n\nThis is a placeholder for the actual web content that would be fetched and converted to Markdown.\n\n## Features\n- URL parsing\n- HTML to Markdown conversion\n- Content extraction\n- Error handling",
            url
        );

        Ok(ToolResult::success(&call.id, self.name(), markdown_content))
    }
}
