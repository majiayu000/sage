use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RememberTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct RememberInput {
    pub memory: String,
}

impl Default for RememberTool {
    fn default() -> Self {
        Self::new()
    }
}

impl RememberTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RememberTool {
    fn name(&self) -> &str {
        "remember"
    }

    fn description(&self) -> &str {
        "Call this tool when user asks you:\n- to remember something\n- to create memory/memories\n\nUse this tool only with information that can be useful in the long-term.\nDo not use this tool for temporary information."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "memory",
                "The concise (1 sentence) memory to remember.",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let memory = call
            .get_string("memory")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'memory' parameter".to_string()))?;

        // TODO: Implement actual memory storage
        // This is a placeholder implementation
        let response = format!("Remembered: {}", memory);

        Ok(ToolResult::success(&call.id, self.name(), response))
    }
}
