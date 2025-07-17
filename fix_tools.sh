#!/bin/bash

# Script to fix all tool implementations to match the new Tool trait

echo "Fixing tool implementations..."

# Fix memory.rs
cat > src/tools/diagnostics/memory.rs << 'EOF'
use sage_core::tools::{Tool, ToolResult, ToolError, ToolCall, ToolSchema, ToolParameter};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RememberTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct RememberInput {
    pub memory: String,
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
            vec![
                ToolParameter::string("memory", "The concise (1 sentence) memory to remember."),
            ]
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let memory = call.get_string("memory")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'memory' parameter".to_string()))?;

        // TODO: Implement actual memory storage
        // This is a placeholder implementation
        let response = format!("Remembered: {}", memory);

        Ok(ToolResult::success(&call.id, self.name(), response))
    }
}
EOF

# Fix mermaid.rs
cat > src/tools/diagnostics/mermaid.rs << 'EOF'
use sage_core::tools::{Tool, ToolResult, ToolError, ToolCall, ToolSchema, ToolParameter};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

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
                ToolParameter::string("diagram_definition", "The Mermaid diagram definition code to render"),
                ToolParameter::string("title", "Optional title for the diagram").with_default("Mermaid Diagram".to_string()).optional(),
            ]
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let diagram_definition = call.get_string("diagram_definition")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'diagram_definition' parameter".to_string()))?;
        
        let title = call.get_string("title").unwrap_or_else(|| "Mermaid Diagram".to_string());

        // TODO: Implement actual Mermaid rendering
        // This is a placeholder implementation
        let rendered_output = format!(
            "# {}\n\n```mermaid\n{}\n```\n\n*Diagram would be rendered interactively in the actual implementation*",
            title, diagram_definition
        );

        Ok(ToolResult::success(&call.id, self.name(), rendered_output))
    }
}
EOF

echo "Fixed memory.rs and mermaid.rs"
EOF
