//! TodoWrite tool - Claude Code compatible task management
//!
//! A simplified task management tool following Claude Code's design.
//! Replaces the entire todo list with each call.

use async_trait::async_trait;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Todo item status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "pending"),
            TodoStatus::InProgress => write!(f, "in_progress"),
            TodoStatus::Completed => write!(f, "completed"),
        }
    }
}

/// A single todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// The imperative form describing what needs to be done
    pub content: String,
    /// Current status of the task
    pub status: TodoStatus,
    /// Present continuous form shown during execution
    #[serde(rename = "activeForm")]
    pub active_form: String,
}

/// Global todo list storage
#[derive(Debug, Default)]
pub struct TodoList {
    todos: RwLock<Vec<TodoItem>>,
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            todos: RwLock::new(Vec::new()),
        }
    }

    /// Replace the entire todo list
    pub fn set_todos(&self, todos: Vec<TodoItem>) {
        let mut list = self.todos.write();
        *list = todos;
    }

    /// Get all todos
    pub fn get_todos(&self) -> Vec<TodoItem> {
        let list = self.todos.read();
        list.clone()
    }

    /// Get the current in-progress task
    pub fn get_current_task(&self) -> Option<TodoItem> {
        let list = self.todos.read();
        list.iter()
            .find(|t| t.status == TodoStatus::InProgress)
            .cloned()
    }

    /// Format todos for display
    pub fn format_display(&self) -> String {
        let list = self.todos.read();
        if list.is_empty() {
            return "No tasks in todo list.".to_string();
        }

        let mut output = String::new();
        for (i, todo) in list.iter().enumerate() {
            let status_icon = match todo.status {
                TodoStatus::Pending => "[ ]",
                TodoStatus::InProgress => "[/]",
                TodoStatus::Completed => "[x]",
            };
            output.push_str(&format!("{}. {} {}\n", i + 1, status_icon, todo.content));
        }
        output
    }

    /// Get completion stats
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let list = self.todos.read();
        let total = list.len();
        let completed = list
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .count();
        let in_progress = list
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        (total, completed, in_progress)
    }
}

// Global todo list instance
pub static GLOBAL_TODO_LIST: Lazy<Arc<TodoList>> = Lazy::new(|| Arc::new(TodoList::new()));

/// TodoWrite tool - Claude Code compatible
pub struct TodoWriteTool {
    todo_list: Arc<TodoList>,
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoWriteTool {
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
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "TodoWrite"
    }

    fn description(&self) -> &str {
        r#"Use this tool to create and manage a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.
It also helps the user understand the progress of the task and overall progress of their requests.

## When to Use This Tool
Use this tool proactively in these scenarios:

1. Complex multi-step tasks - When a task requires 3 or more distinct steps or actions
2. Non-trivial and complex tasks - Tasks that require careful planning or multiple operations
3. User explicitly requests todo list - When the user directly asks you to use the todo list
4. User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
5. After receiving new instructions - Immediately capture user requirements as todos
6. When you start working on a task - Mark it as in_progress BEFORE beginning work. Ideally you should only have one todo as in_progress at a time
7. After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation

## When NOT to Use This Tool

Skip using this tool when:
1. There is only a single, straightforward task
2. The task is trivial and tracking it provides no organizational benefit
3. The task can be completed in less than 3 trivial steps
4. The task is purely conversational or informational

NOTE that you should not use this tool if there is only one trivial task to do. In this case you are better off just doing the task directly.

## Examples of When to Use the Todo List

<example>
User: I want to add a dark mode toggle to the application settings. Make sure you run the tests and build when you're done!
Assistant: I'll help add a dark mode toggle to your application settings. Let me create a todo list to track this implementation.
*Creates todo list with the following items:*
1. Creating dark mode toggle component in Settings page
2. Adding dark mode state management (context/store)
3. Implementing CSS-in-JS styles for dark theme
4. Updating existing components to support theme switching
5. Running tests and build process, addressing any failures or errors that occur
</example>

<example>
User: Help me rename the function getCwd to getCurrentWorkingDirectory across my project
Assistant: Let me first search through your codebase to find all occurrences of 'getCwd'.
*Uses grep or search tools to locate all instances*
Assistant: I've found 15 instances across 8 different files. Let me create a todo list to track these changes.
*Creates todo list with specific items for each file*
</example>

## Examples of When NOT to Use the Todo List

<example>
User: How do I print 'Hello World' in Python?
Assistant: [Just answers directly without using TodoWrite - single trivial task]
</example>

<example>
User: Can you add a comment to the calculateTotal function?
Assistant: [Just edits the file directly without using TodoWrite - single straightforward task]
</example>

## Task States and Management

1. **Task States**: Use these states to track progress:
   - pending: Task not yet started
   - in_progress: Currently working on (limit to ONE task at a time)
   - completed: Task finished successfully

   **IMPORTANT**: Task descriptions must have two forms:
   - content: The imperative form describing what needs to be done (e.g., "Run tests", "Build the project")
   - activeForm: The present continuous form shown during execution (e.g., "Running tests", "Building the project")

2. **Task Management**:
   - Update task status in real-time as you work
   - Mark tasks complete IMMEDIATELY after finishing (don't batch completions)
   - Exactly ONE task must be in_progress at any time (not less, not more)
   - Complete current tasks before starting new ones
   - Remove tasks that are no longer relevant from the list entirely

3. **Task Completion Requirements**:
   - ONLY mark a task as completed when you have FULLY accomplished it
   - If you encounter errors, blockers, or cannot finish, keep the task as in_progress
   - When blocked, create a new task describing what needs to be resolved
   - Never mark a task as completed if:
     - Tests are failing
     - Implementation is partial
     - You encountered unresolved errors
     - You couldn't find necessary files or dependencies

4. **Task Breakdown**:
   - Create specific, actionable items
   - Break complex tasks into smaller, manageable steps
   - Use clear, descriptive task names
   - Always provide both forms:
     - content: "Fix authentication bug"
     - activeForm: "Fixing authentication bug"

When in doubt, use this tool. Being proactive with task management demonstrates attentiveness and ensures you complete all requirements successfully."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "description": "The updated todo list. Each item must have content (imperative form), status (pending/in_progress/completed), and activeForm (present continuous form).",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": {
                                    "type": "string",
                                    "description": "The imperative form describing what needs to be done (e.g., 'Run tests', 'Build the project')",
                                    "minLength": 1
                                },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"],
                                    "description": "Current status of the task"
                                },
                                "activeForm": {
                                    "type": "string",
                                    "description": "Present continuous form shown during execution (e.g., 'Running tests', 'Building the project')",
                                    "minLength": 1
                                }
                            },
                            "required": ["content", "status", "activeForm"]
                        }
                    }
                },
                "required": ["todos"]
            }),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Parse the todos array from input
        let todos_value = call
            .arguments
            .get("todos")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'todos' parameter".to_string()))?;

        let todos: Vec<TodoItem> = serde_json::from_value(todos_value.clone())
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid todos format: {}", e)))?;

        // Validate todos
        let mut in_progress_count = 0;
        for todo in &todos {
            if todo.content.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "Todo content cannot be empty".to_string(),
                ));
            }
            if todo.active_form.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "Todo activeForm cannot be empty".to_string(),
                ));
            }
            if todo.status == TodoStatus::InProgress {
                in_progress_count += 1;
            }
        }

        // Warn if more than one task is in_progress (but don't fail)
        let warning = if in_progress_count > 1 {
            Some(format!(
                "Warning: {} tasks are marked as in_progress. Ideally only one task should be in_progress at a time.",
                in_progress_count
            ))
        } else {
            None
        };

        // Update the todo list
        self.todo_list.set_todos(todos);

        // Get stats
        let (total, completed, in_progress) = self.todo_list.get_stats();
        let current_task = self.todo_list.get_current_task();

        // Format response
        let mut response = format!(
            "Todos have been modified successfully. {} total, {} completed, {} in progress.",
            total, completed, in_progress
        );

        if let Some(task) = current_task {
            response.push_str(&format!("\n\nCurrent task: {}", task.active_form));
        }

        if let Some(warn) = warning {
            response.push_str(&format!("\n\n{}", warn));
        }

        response.push_str("\n\nEnsure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable");

        // Build result using standardized format
        let mut result = ToolResult::success(&call.id, self.name(), response);

        // Add metadata about todo list state
        result = result
            .with_metadata("total_tasks", serde_json::json!(total))
            .with_metadata("completed_tasks", serde_json::json!(completed))
            .with_metadata("in_progress_tasks", serde_json::json!(in_progress));

        Ok(result)
    }
}

/// Get the current todo list for display (used by UI)
pub fn get_current_todos() -> Vec<TodoItem> {
    GLOBAL_TODO_LIST.get_todos()
}

/// Get formatted todo list string
pub fn get_todo_display() -> String {
    GLOBAL_TODO_LIST.format_display()
}

/// Get the current in-progress task
pub fn get_current_task() -> Option<TodoItem> {
    GLOBAL_TODO_LIST.get_current_task()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_todo_write_basic() {
        let tool = TodoWriteTool::with_list(Arc::new(TodoList::new()));

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TodoWrite".to_string(),
            arguments: json!({
                "todos": [
                    {
                        "content": "Implement feature A",
                        "status": "in_progress",
                        "activeForm": "Implementing feature A"
                    },
                    {
                        "content": "Write tests",
                        "status": "pending",
                        "activeForm": "Writing tests"
                    }
                ]
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("2 total"));
        assert!(output.contains("1 in progress"));
    }

    #[tokio::test]
    async fn test_todo_write_completion() {
        let list = Arc::new(TodoList::new());
        let tool = TodoWriteTool::with_list(list.clone());

        // Add initial todos
        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TodoWrite".to_string(),
            arguments: json!({
                "todos": [
                    {
                        "content": "Task 1",
                        "status": "completed",
                        "activeForm": "Completing task 1"
                    },
                    {
                        "content": "Task 2",
                        "status": "in_progress",
                        "activeForm": "Working on task 2"
                    },
                    {
                        "content": "Task 3",
                        "status": "pending",
                        "activeForm": "Starting task 3"
                    }
                ]
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("1 completed"));

        let (total, completed, in_progress) = list.get_stats();
        assert_eq!(total, 3);
        assert_eq!(completed, 1);
        assert_eq!(in_progress, 1);
    }

    #[tokio::test]
    async fn test_todo_write_empty_content_error() {
        let tool = TodoWriteTool::with_list(Arc::new(TodoList::new()));

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TodoWrite".to_string(),
            arguments: json!({
                "todos": [
                    {
                        "content": "",
                        "status": "pending",
                        "activeForm": "Doing something"
                    }
                ]
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_todo_display_format() {
        let list = TodoList::new();
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

        let display = list.format_display();
        assert!(display.contains("[x] Task 1"));
        assert!(display.contains("[/] Task 2"));
        assert!(display.contains("[ ] Task 3"));
    }
}
