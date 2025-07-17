use sage_core::tools::{Tool, ToolResult, ToolError, ToolCall, ToolSchema, ToolParameter};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct DiagnosticsTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticsInput {
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticIssue {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub message: String,
    pub code: Option<String>,
}

impl Default for DiagnosticsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DiagnosticsTool {
    fn name(&self) -> &str {
        "diagnostics"
    }

    fn description(&self) -> &str {
        "Get issues (errors, warnings, etc.) from the IDE. You must provide the paths of the files for which you want to get issues."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("paths", "Required list of file paths to get issues for from the IDE."),
            ]
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let paths: Vec<String> = call.get_argument("paths")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'paths' parameter".to_string()))?;

        // TODO: Implement actual IDE diagnostics integration
        // This is a placeholder implementation
        let mut issues = Vec::new();

        for path in &paths {
            // Simulate some diagnostic issues
            issues.push(DiagnosticIssue {
                file: path.clone(),
                line: 1,
                column: 1,
                severity: "info".to_string(),
                message: "No issues found".to_string(),
                code: None,
            });
        }

        let output = serde_json::to_string_pretty(&issues)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize output: {}", e)))?;

        Ok(ToolResult::success(&call.id, self.name(), output))
    }
}
