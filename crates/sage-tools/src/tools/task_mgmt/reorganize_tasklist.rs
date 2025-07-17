//! Reorganize tasklist tool for complex restructuring

use async_trait::async_trait;
use serde_json::json;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use crate::tools::task_mgmt::task_management::{GLOBAL_TASK_LIST, Task, TaskState};
use uuid::Uuid;

/// Tool for reorganizing the task list structure
pub struct ReorganizeTasklistTool;

impl Default for ReorganizeTasklistTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReorganizeTasklistTool {
    pub fn new() -> Self {
        Self
    }

    /// Parse markdown task list format
    fn parse_markdown_tasklist(&self, markdown: &str) -> Result<Vec<ParsedTask>, ToolError> {
        let mut tasks = Vec::new();
        let _current_indent = 0;
        let mut parent_stack: Vec<String> = Vec::new();

        for line in markdown.lines() {
            let trimmed = line.trim();
            
            // Skip empty lines and headers
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse task line: "- [x] UUID:abc NAME:task DESCRIPTION:desc"
            if let Some(task) = self.parse_task_line(line)? {
                let line_indent = line.len() - line.trim_start().len();
                
                // Adjust parent stack based on indentation
                while parent_stack.len() > (line_indent / 2) {
                    parent_stack.pop();
                }

                let parent_id = parent_stack.last().cloned();
                
                let mut parsed_task = task;
                parsed_task.parent_id = parent_id;
                parsed_task.indent_level = line_indent / 2;

                // Add to parent stack if this could be a parent
                parent_stack.push(parsed_task.id.clone());
                
                tasks.push(parsed_task);
            }
        }

        Ok(tasks)
    }

    /// Parse a single task line
    fn parse_task_line(&self, line: &str) -> Result<Option<ParsedTask>, ToolError> {
        let trimmed = line.trim();
        
        // Check if it's a task line (starts with "- ")
        if !trimmed.starts_with("- ") {
            return Ok(None);
        }

        // Parse state
        let state = if trimmed.starts_with("- [ ]") {
            TaskState::NotStarted
        } else if trimmed.starts_with("- [/]") {
            TaskState::InProgress
        } else if trimmed.starts_with("- [-]") {
            TaskState::Cancelled
        } else if trimmed.starts_with("- [x]") {
            TaskState::Complete
        } else {
            return Err(ToolError::InvalidArguments("Invalid task state format".to_string()));
        };

        // Remove state prefix
        let content = trimmed.strip_prefix("- [").unwrap()
            .strip_prefix(&format!("{}]", match state {
                TaskState::NotStarted => " ",
                TaskState::InProgress => "/",
                TaskState::Cancelled => "-",
                TaskState::Complete => "x",
            })).unwrap().trim();

        // Parse UUID, NAME, DESCRIPTION
        let mut uuid = String::new();
        let mut name = String::new();
        let mut description = String::new();

        let parts: Vec<&str> = content.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            if parts[i].starts_with("UUID:") {
                uuid = parts[i].strip_prefix("UUID:").unwrap().to_string();
            } else if parts[i].starts_with("NAME:") {
                // Collect name until next field
                let mut name_parts = vec![parts[i].strip_prefix("NAME:").unwrap()];
                i += 1;
                while i < parts.len() && !parts[i].starts_with("DESCRIPTION:") {
                    name_parts.push(parts[i]);
                    i += 1;
                }
                name = name_parts.join(" ");
                continue; // Skip the i += 1 at the end
            } else if parts[i].starts_with("DESCRIPTION:") {
                // Collect description until end
                let mut desc_parts = vec![parts[i].strip_prefix("DESCRIPTION:").unwrap()];
                i += 1;
                while i < parts.len() {
                    desc_parts.push(parts[i]);
                    i += 1;
                }
                description = desc_parts.join(" ");
                break;
            }
            i += 1;
        }

        // Generate new UUID if "NEW_UUID" or empty
        if uuid.is_empty() || uuid == "NEW_UUID" {
            uuid = Uuid::new_v4().to_string();
        }

        Ok(Some(ParsedTask {
            id: uuid,
            name,
            description,
            state,
            parent_id: None,
            indent_level: 0,
        }))
    }
}

#[derive(Debug, Clone)]
struct ParsedTask {
    id: String,
    name: String,
    description: String,
    state: TaskState,
    parent_id: Option<String>,
    indent_level: usize,
}

#[async_trait]
impl Tool for ReorganizeTasklistTool {
    fn name(&self) -> &str {
        "reorganize_tasklist"
    }

    fn description(&self) -> &str {
        "Reorganize the task list structure for the current conversation. Use this only for major restructuring like reordering tasks, changing hierarchy. For individual task updates, use update_tasks tool."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let markdown = tool_call.arguments
            .get("markdown")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing required parameter: markdown".to_string()))?;

        // Parse the markdown
        let parsed_tasks = self.parse_markdown_tasklist(markdown)?;

        if parsed_tasks.is_empty() {
            return Ok(ToolResult::error(&tool_call.id, self.name(), "No valid tasks found in markdown"));
        }

        // Convert parsed tasks to Task objects
        let mut new_tasks = Vec::new();
        for parsed_task in &parsed_tasks {
            let mut task = Task::new(parsed_task.name.clone(), parsed_task.description.clone());
            task.id = parsed_task.id.clone();
            task.state = parsed_task.state.clone();
            task.parent_id = parsed_task.parent_id.clone();
            new_tasks.push(task);
        }

        // Clear existing tasks and rebuild
        GLOBAL_TASK_LIST.clear_and_rebuild(new_tasks)?;

        Ok(ToolResult::success(&tool_call.id, self.name(), format!(
            "Task list reorganized successfully. {} tasks processed.",
            parsed_tasks.len()
        )))
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
                        "description": "The markdown representation of the task list to update. Should be in the format specified by the view_tasklist tool. New tasks should have a UUID of 'NEW_UUID'. Must contain exactly one root task with proper hierarchy using dash indentation."
                    }
                },
                "required": ["markdown"]
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::tools::task_mgmt::task_management::GLOBAL_TASK_LIST;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_reorganize_tasklist() {
        // Clear any existing tasks first
        let root_tasks = GLOBAL_TASK_LIST.get_root_task_ids();
        if !root_tasks.is_empty() {
            // Skip test if there are existing tasks to avoid interference
            return;
        }

        // First add some tasks
        let markdown = r#"
# Task List

- [ ] UUID:NEW_UUID NAME:Root Task DESCRIPTION:This is the root task
  - [ ] UUID:NEW_UUID NAME:Subtask 1 DESCRIPTION:This is the first subtask
  - [/] UUID:NEW_UUID NAME:Subtask 2 DESCRIPTION:This is the second subtask
    - [x] UUID:NEW_UUID NAME:Sub-subtask DESCRIPTION:This is a sub-subtask
"#;

        let tool = ReorganizeTasklistTool::new();
        let call = create_tool_call("test-1", "reorganize_tasklist", json!({
            "markdown": markdown
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Task list reorganized successfully"));

        // Verify the task list structure
        let root_tasks = GLOBAL_TASK_LIST.get_root_task_ids();
        assert_eq!(root_tasks.len(), 1);

        // Clean up
        GLOBAL_TASK_LIST.clear_and_rebuild(vec![]).unwrap();
    }
}
