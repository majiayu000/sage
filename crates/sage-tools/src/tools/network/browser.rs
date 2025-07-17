use sage_core::tools::{Tool, ToolResult, ToolError, ToolCall, ToolSchema, ToolParameter};
use serde::{Deserialize, Serialize};
use std::process::Command;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct BrowserTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserInput {
    pub url: String,
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "open-browser"
    }

    fn description(&self) -> &str {
        "Open a URL in the default browser.\n\n1. The tool takes in a URL and opens it in the default browser.\n2. The tool does not return any content. It is intended for the user to visually inspect and interact with the page. You will not have access to it.\n3. You should not use `open-browser` on a URL that you have called the tool on before in the conversation history, because the page is already open in the user's browser and the user can see it and refresh it themselves. Each time you call `open-browser`, it will jump the user to the browser window, which is highly annoying to the user."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("url", "The URL to open in the browser."),
            ]
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let url = call.get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        // Open URL in default browser
        let result = if cfg!(target_os = "macos") {
            Command::new("open").arg(&url).output()
        } else if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", "start", &url]).output()
        } else {
            Command::new("xdg-open").arg(&url).output()
        };

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(ToolResult::success(&call.id, self.name(), format!("Opened {} in default browser", url)))
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(ToolError::ExecutionFailed(format!("Failed to open browser: {}", error)))
                }
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!("Failed to execute browser command: {}", e))),
        }
    }
}
