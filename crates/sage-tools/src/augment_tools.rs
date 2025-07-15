//! Sage Agent specific tools implementation

use async_trait::async_trait;
use serde_json::json;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};

/// Simple codebase retrieval tool
pub struct CodebaseRetrievalTool;

impl CodebaseRetrievalTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for CodebaseRetrievalTool {
    fn name(&self) -> &str {
        "codebase-retrieval"
    }

    fn description(&self) -> &str {
        "Sage's context engine, the world's best codebase context engine. It takes in a natural language description of the code you are looking for and uses a proprietary retrieval/embedding model suite that produces the highest-quality recall of relevant code snippets from across the codebase."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let information_request = tool_call.arguments
            .get("information_request")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing required parameter: information_request".to_string()))?;

        // Simple implementation - search for files containing keywords
        let keywords: Vec<&str> = information_request.split_whitespace().collect();
        let mut results = Vec::new();

        // Search current directory for relevant files
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(entries) = std::fs::read_dir(&current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(extension) = path.extension() {
                            if matches!(extension.to_str(), Some("rs") | Some("py") | Some("js") | Some("ts") | Some("java") | Some("cpp") | Some("c") | Some("h") | Some("go") | Some("rb")) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let content_lower = content.to_lowercase();
                                    if keywords.iter().any(|keyword| content_lower.contains(&keyword.to_lowercase())) {
                                        let relative_path = path.strip_prefix(&current_dir)
                                            .unwrap_or(&path)
                                            .to_string_lossy();
                                        
                                        // Extract first few lines that contain keywords
                                        let matching_lines: Vec<String> = content
                                            .lines()
                                            .enumerate()
                                            .filter(|(_, line)| {
                                                let line_lower = line.to_lowercase();
                                                keywords.iter().any(|keyword| line_lower.contains(&keyword.to_lowercase()))
                                            })
                                            .take(3)
                                            .map(|(i, line)| format!("{:4}: {}", i + 1, line))
                                            .collect();
                                        
                                        if !matching_lines.is_empty() {
                                            results.push(format!(
                                                "File: {}\n{}",
                                                relative_path,
                                                matching_lines.join("\n")
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let response = if results.is_empty() {
            format!("No relevant code snippets found for: {}", information_request)
        } else {
            format!("Found relevant code snippets:\n\n{}", results.join("\n\n"))
        };

        Ok(ToolResult::success(&tool_call.id, self.name(), response))
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
                        "description": "A description of the information you need."
                    }
                },
                "required": ["information_request"]
            }),
        }
    }
}

/// Simple task management placeholder tools
pub struct ViewTasklistTool;

impl ViewTasklistTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ViewTasklistTool {
    fn name(&self) -> &str {
        "view_tasklist"
    }

    fn description(&self) -> &str {
        "View the current task list for the conversation."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let response = "No tasks in the current task list.\n\nNote: Task management tools are placeholder implementations. For full functionality, use the existing sequential_thinking tool for complex planning.";
        Ok(ToolResult::success(&tool_call.id, self.name(), response))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }
}

/// Add tasks tool (placeholder)
pub struct AddTasksTool;

impl AddTasksTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for AddTasksTool {
    fn name(&self) -> &str {
        "add_tasks"
    }

    fn description(&self) -> &str {
        "Add one or more new tasks to the task list. Can add a single task or multiple tasks in one call."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let response = "Task management is currently a placeholder implementation.\n\nFor complex planning and task breakdown, please use the 'sequential_thinking' tool which provides structured problem-solving capabilities.";
        Ok(ToolResult::success(&tool_call.id, self.name(), response))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "description": "Array of tasks to create.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "description": {"type": "string"}
                            },
                            "required": ["name", "description"]
                        }
                    }
                },
                "required": ["tasks"]
            }),
        }
    }
}

/// Update tasks tool (placeholder)
pub struct UpdateTasksTool;

impl UpdateTasksTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for UpdateTasksTool {
    fn name(&self) -> &str {
        "update_tasks"
    }

    fn description(&self) -> &str {
        "Update one or more tasks' properties (state, name, description)."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let response = "Task management is currently a placeholder implementation.\n\nFor tracking progress and managing complex work, please use the 'sequential_thinking' tool.";
        Ok(ToolResult::success(&tool_call.id, self.name(), response))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "description": "Array of tasks to update.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "task_id": {"type": "string"},
                                "state": {"type": "string", "enum": ["NOT_STARTED", "IN_PROGRESS", "CANCELLED", "COMPLETE"]}
                            },
                            "required": ["task_id"]
                        }
                    }
                },
                "required": ["tasks"]
            }),
        }
    }
}

/// Reorganize tasklist tool (placeholder)
pub struct ReorganizeTasklistTool;

impl ReorganizeTasklistTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReorganizeTasklistTool {
    fn name(&self) -> &str {
        "reorganize_tasklist"
    }

    fn description(&self) -> &str {
        "Reorganize the task list structure for the current conversation."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let response = "Task list reorganization is currently a placeholder implementation.\n\nFor complex task restructuring and planning, please use the 'sequential_thinking' tool.";
        Ok(ToolResult::success(&tool_call.id, self.name(), response))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "markdown": {
                        "type": "string",
                        "description": "The markdown representation of the task list to update."
                    }
                },
                "required": ["markdown"]
            }),
        }
    }
}
