//! Task completion tool

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde_json::json;
use std::sync::Arc;

use super::todo_write::{GLOBAL_TODO_LIST, TodoList};

/// Tool for marking tasks as completed
pub struct TaskDoneTool {
    todo_list: Arc<TodoList>,
}

impl TaskDoneTool {
    /// Create a new task done tool
    pub fn new() -> Self {
        Self {
            todo_list: GLOBAL_TODO_LIST.clone(),
        }
    }

    /// Create a task done tool with a custom todo list (for tests)
    pub fn with_list(todo_list: Arc<TodoList>) -> Self {
        Self { todo_list }
    }
}

impl Default for TaskDoneTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskDoneTool {
    fn name(&self) -> &str {
        "TaskDone"
    }

    fn description(&self) -> &str {
        "Use this tool ONLY when you have FULLY completed the assigned task with WORKING CODE. \
         DO NOT call this if you have only written plans, designs, or documentation. \
         A task is complete ONLY when: (1) Code files have been created or modified, \
         (2) The implementation is functional, (3) Tests pass (if applicable). \
         Provide a summary of what code was written and how it works."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "summary",
                    "A summary of what was accomplished in completing the task",
                ),
                ToolParameter::optional_string(
                    "details",
                    "Additional details about the task completion",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let summary = call.get_string("summary").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'summary' parameter".to_string())
        })?;

        if summary.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Summary cannot be empty".to_string(),
            ));
        }

        let details = call.get_string("details").unwrap_or_default();

        let mut completion_message =
            format!("✅ Task Completed Successfully!\n\nSummary: {}", summary);

        if !details.trim().is_empty() {
            completion_message.push_str(&format!("\n\nDetails:\n{}", details));
        }

        // Keep completion signal aligned with todo state transitions.
        let completed_task = self.todo_list.complete_current_task();
        if let Some(task) = &completed_task {
            completion_message.push_str(&format!("\n\nMarked todo as completed: {}", task.content));
        }

        if let Some(next_task) = self.todo_list.get_current_task() {
            completion_message.push_str(&format!(
                "\nNext task in progress: {}",
                next_task.active_form
            ));
        }

        // Add timestamp
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        completion_message.push_str(&format!("\n\nCompleted at: {}", timestamp));

        let mut result = ToolResult::success(&call.id, self.name(), completion_message);
        result = result
            .with_metadata("todo_state_updated", json!(completed_task.is_some()))
            .with_metadata(
                "completed_todo",
                json!(completed_task.as_ref().map(|t| &t.content)),
            );

        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let summary = call.get_string("summary").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'summary' parameter".to_string())
        })?;

        if summary.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Summary cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(5)) // 5 seconds - this is a very lightweight operation
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Task completion doesn't interfere with other operations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::task_mgmt::todo_write::{TodoItem, TodoStatus};
    use serde_json::json;
    use std::collections::HashMap;

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
    async fn test_task_done_basic() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call(
            "test-1",
            "TaskDone",
            json!({
                "summary": "Successfully implemented the user authentication system"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("✅ Task Completed Successfully!"));
        assert!(output.contains("Successfully implemented the user authentication system"));
        assert!(output.contains("Completed at:"));
    }

    #[tokio::test]
    async fn test_task_done_with_details() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call(
            "test-2",
            "TaskDone",
            json!({
                "summary": "Fixed the database connection issue",
                "details": "Updated the connection string and added proper error handling. All tests are now passing."
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("✅ Task Completed Successfully!"));
        assert!(output.contains("Fixed the database connection issue"));
        assert!(output.contains("Details:"));
        assert!(output.contains("Updated the connection string"));
        assert!(output.contains("Completed at:"));
    }

    #[tokio::test]
    async fn test_task_done_empty_summary() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call(
            "test-3",
            "TaskDone",
            json!({
                "summary": ""
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Summary cannot be empty"));
    }

    #[tokio::test]
    async fn test_task_done_whitespace_summary() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call(
            "test-4",
            "TaskDone",
            json!({
                "summary": "   \n\t  \n   "
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Summary cannot be empty"));
    }

    #[tokio::test]
    async fn test_task_done_missing_summary() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call("test-5", "TaskDone", json!({}));

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing") || err.to_string().contains("summary"));
    }

    #[tokio::test]
    async fn test_task_done_empty_details() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call(
            "test-6",
            "TaskDone",
            json!({
                "summary": "Task completed",
                "details": ""
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Task completed"));
        assert!(!output.contains("Details:"));
    }

    #[tokio::test]
    async fn test_task_done_whitespace_details() {
        let tool = TaskDoneTool::new();
        let call = create_tool_call(
            "test-7",
            "TaskDone",
            json!({
                "summary": "Task completed",
                "details": "   \n\t  \n   "
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Task completed"));
        assert!(!output.contains("Details:"));
    }

    #[tokio::test]
    async fn test_task_done_validation() {
        let tool = TaskDoneTool::new();

        let call = create_tool_call(
            "test-8",
            "TaskDone",
            json!({
                "summary": ""
            }),
        );
        let validation_result = tool.validate(&call);
        assert!(validation_result.is_err());
        assert!(
            validation_result
                .unwrap_err()
                .to_string()
                .contains("Summary cannot be empty")
        );
    }

    #[test]
    fn test_task_done_schema() {
        let tool = TaskDoneTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "TaskDone");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_task_done_max_execution_duration() {
        use std::time::Duration;
        let tool = TaskDoneTool::new();
        assert_eq!(tool.max_execution_duration(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_task_done_supports_parallel_execution() {
        let tool = TaskDoneTool::new();
        assert!(tool.supports_parallel_execution());
    }

    #[tokio::test]
    async fn test_task_done_marks_current_todo_completed() {
        let list = Arc::new(TodoList::new());
        list.set_todos(vec![
            TodoItem {
                content: "Step 1".to_string(),
                status: TodoStatus::InProgress,
                active_form: "Working on step 1".to_string(),
            },
            TodoItem {
                content: "Step 2".to_string(),
                status: TodoStatus::Pending,
                active_form: "Working on step 2".to_string(),
            },
        ]);

        let tool = TaskDoneTool::with_list(Arc::clone(&list));
        let call = create_tool_call(
            "test-state",
            "TaskDone",
            json!({
                "summary": "Completed step 1"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        let todos = list.get_todos();
        assert_eq!(todos[0].status, TodoStatus::Completed);
        assert_eq!(todos[1].status, TodoStatus::InProgress);
    }
}
