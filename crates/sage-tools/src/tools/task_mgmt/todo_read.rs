//! TodoRead tool - Read current task list status
//!
//! Provides a read-only view of the current todo list, allowing agents
//! to check task progress without modifying the list.

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;
use std::sync::Arc;

use super::todo_write::{TodoList, TodoStatus, GLOBAL_TODO_LIST};

/// TodoRead tool - Read current task list
pub struct TodoReadTool {
    todo_list: Arc<TodoList>,
}

impl Default for TodoReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoReadTool {
    pub fn new() -> Self {
        Self {
            todo_list: GLOBAL_TODO_LIST.clone(),
        }
    }

    pub fn with_list(todo_list: Arc<TodoList>) -> Self {
        Self { todo_list }
    }
}

#[async_trait]
impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        "TodoRead"
    }

    fn description(&self) -> &str {
        "Read the current task list to check progress and status. Use this to review tasks before making updates."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Optional filter: 'all' (default), 'pending', 'in_progress', 'completed'",
                        "enum": ["all", "pending", "in_progress", "completed"],
                        "default": "all"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let filter = call
            .arguments
            .get("filter")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let todos = self.todo_list.get_todos();
        let (total, completed, in_progress) = self.todo_list.get_stats();

        // Filter todos based on filter parameter
        let filtered_todos: Vec<_> = todos
            .iter()
            .filter(|t| match filter {
                "pending" => t.status == TodoStatus::Pending,
                "in_progress" => t.status == TodoStatus::InProgress,
                "completed" => t.status == TodoStatus::Completed,
                _ => true, // "all"
            })
            .collect();

        // Format output
        let mut output = String::new();

        if todos.is_empty() {
            output.push_str("No tasks in todo list.\n\n");
            output.push_str("Use TodoWrite to create tasks when working on complex multi-step work.");
        } else {
            output.push_str(&format!(
                "## Todo List Status\n\n**Total**: {} | **Completed**: {} | **In Progress**: {} | **Pending**: {}\n\n",
                total,
                completed,
                in_progress,
                total - completed - in_progress
            ));

            if filter != "all" {
                output.push_str(&format!("**Filter**: {}\n\n", filter));
            }

            output.push_str("### Tasks\n\n");
            for (i, todo) in filtered_todos.iter().enumerate() {
                let status_icon = match todo.status {
                    TodoStatus::Pending => "[ ]",
                    TodoStatus::InProgress => "[/]",
                    TodoStatus::Completed => "[x]",
                };
                output.push_str(&format!("{}. {} {}\n", i + 1, status_icon, todo.content));
            }

            // Show current task if any
            if let Some(current) = self.todo_list.get_current_task() {
                output.push_str(&format!("\n**Current Task**: {}", current.active_form));
            }
        }

        let mut result = ToolResult::success(&call.id, self.name(), output);

        // Add metadata
        result = result
            .with_metadata("total_tasks", json!(total))
            .with_metadata("completed_tasks", json!(completed))
            .with_metadata("in_progress_tasks", json!(in_progress))
            .with_metadata("filter", json!(filter));

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::todo_write::TodoItem;

    #[tokio::test]
    async fn test_todo_read_empty() {
        let tool = TodoReadTool::with_list(Arc::new(TodoList::new()));

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TodoRead".to_string(),
            arguments: std::collections::HashMap::new(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("No tasks"));
    }

    #[tokio::test]
    async fn test_todo_read_with_tasks() {
        let list = Arc::new(TodoList::new());
        list.set_todos(vec![
            TodoItem {
                content: "Task 1".to_string(),
                status: TodoStatus::Completed,
                active_form: "Completing task 1".to_string(),
            },
            TodoItem {
                content: "Task 2".to_string(),
                status: TodoStatus::InProgress,
                active_form: "Working on task 2".to_string(),
            },
            TodoItem {
                content: "Task 3".to_string(),
                status: TodoStatus::Pending,
                active_form: "Starting task 3".to_string(),
            },
        ]);

        let tool = TodoReadTool::with_list(list);

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TodoRead".to_string(),
            arguments: std::collections::HashMap::new(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("Total"));
        assert!(output.contains("Task 1"));
        assert!(output.contains("[x]"));
        assert!(output.contains("[/]"));
        assert!(output.contains("[ ]"));
        assert!(output.contains("Current Task"));
    }

    #[tokio::test]
    async fn test_todo_read_with_filter() {
        let list = Arc::new(TodoList::new());
        list.set_todos(vec![
            TodoItem {
                content: "Done task".to_string(),
                status: TodoStatus::Completed,
                active_form: "Done".to_string(),
            },
            TodoItem {
                content: "Working task".to_string(),
                status: TodoStatus::InProgress,
                active_form: "Working".to_string(),
            },
        ]);

        let tool = TodoReadTool::with_list(list);

        let mut args = std::collections::HashMap::new();
        args.insert("filter".to_string(), json!("completed"));

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TodoRead".to_string(),
            arguments: args,
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("Done task"));
        assert!(output.contains("**Filter**: completed"));
    }
}
