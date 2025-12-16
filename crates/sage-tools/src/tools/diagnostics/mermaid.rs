use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RenderMermaidTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderMermaidInput {
    pub diagram_definition: String,
    #[serde(default = "default_title")]
    pub title: String,
}

fn default_title() -> String {
    "Mermaid Diagram".to_string()
}

impl Default for RenderMermaidTool {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderMermaidTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RenderMermaidTool {
    fn name(&self) -> &str {
        "render-mermaid"
    }

    fn description(&self) -> &str {
        "Render a Mermaid diagram from the provided definition. This tool takes Mermaid diagram code and renders it as an interactive diagram with pan/zoom controls and copy functionality."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "diagram_definition",
                    "The Mermaid diagram definition code to render",
                ),
                ToolParameter::string("title", "Optional title for the diagram")
                    .with_default("Mermaid Diagram".to_string())
                    .optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let diagram_definition = call.get_string("diagram_definition").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'diagram_definition' parameter".to_string())
        })?;

        let title = call
            .get_string("title")
            .unwrap_or_else(|| "Mermaid Diagram".to_string());

        // TODO: Implement actual Mermaid rendering
        // This is a placeholder implementation
        let rendered_output = format!(
            "# {}\n\n```mermaid\n{}\n```\n\n*Diagram would be rendered interactively in the actual implementation*",
            title, diagram_definition
        );

        Ok(ToolResult::success(&call.id, self.name(), rendered_output))
    }
}
