//! Task management tools for organizing complex work

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use uuid::Uuid;

/// Task state enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskState {
    #[serde(rename = "NOT_STARTED")]
    NotStarted,
    #[serde(rename = "IN_PROGRESS")]
    InProgress,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "COMPLETE")]
    Complete,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::NotStarted => write!(f, "[ ]"),
            TaskState::InProgress => write!(f, "[/]"),
            TaskState::Cancelled => write!(f, "[-]"),
            TaskState::Complete => write!(f, "[x]"),
        }
    }
}

/// Individual task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub state: TaskState,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Task {
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            state: TaskState::NotStarted,
            parent_id: None,
            children: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Task list manager
#[derive(Debug, Clone)]
pub struct TaskList {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
    root_tasks: Arc<Mutex<Vec<String>>>,
}

impl Default for TaskList {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskList {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            root_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_task(&self, mut task: Task, parent_id: Option<String>, after_task_id: Option<String>) -> Result<String, ToolError> {
        let mut tasks = self.tasks.lock().unwrap();
        let mut root_tasks = self.root_tasks.lock().unwrap();

        task.parent_id = parent_id.clone();
        let task_id = task.id.clone();

        if let Some(parent_id) = &parent_id {
            // Add as subtask
            if let Some(parent) = tasks.get_mut(parent_id) {
                parent.children.push(task_id.clone());
            } else {
                return Err(ToolError::InvalidArguments(format!("Parent task not found: {}", parent_id)));
            }
        } else {
            // Add as root task
            if let Some(after_id) = after_task_id {
                if let Some(pos) = root_tasks.iter().position(|id| id == &after_id) {
                    root_tasks.insert(pos + 1, task_id.clone());
                } else {
                    root_tasks.push(task_id.clone());
                }
            } else {
                root_tasks.push(task_id.clone());
            }
        }

        tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }

    pub fn update_task(&self, task_id: &str, name: Option<String>, description: Option<String>, state: Option<TaskState>) -> Result<(), ToolError> {
        let mut tasks = self.tasks.lock().unwrap();
        
        if let Some(task) = tasks.get_mut(task_id) {
            if let Some(name) = name {
                task.name = name;
            }
            if let Some(description) = description {
                task.description = description;
            }
            if let Some(state) = state {
                task.state = state;
            }
            task.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ToolError::InvalidArguments(format!("Task not found: {}", task_id)))
        }
    }

    pub fn view_tasklist(&self) -> String {
        // Handle poisoned mutex by recovering the data
        let tasks = match self.tasks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let root_tasks = match self.root_tasks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if root_tasks.is_empty() {
            return "No tasks in the current task list.".to_string();
        }

        let mut output = String::from("# Current Task List\n\n");

        for root_id in root_tasks.iter() {
            if let Some(task) = tasks.get(root_id) {
                self.format_task(&tasks, task, 0, &mut output);
            }
        }

        output
    }

    pub fn get_root_task_ids(&self) -> Vec<String> {
        let root_tasks = self.root_tasks.lock().unwrap();
        root_tasks.clone()
    }

    pub fn clear_and_rebuild(&self, new_tasks: Vec<Task>) -> Result<(), ToolError> {
        let mut tasks = self.tasks.lock().unwrap();
        let mut root_tasks = self.root_tasks.lock().unwrap();

        tasks.clear();
        root_tasks.clear();

        // Add all tasks
        for task in new_tasks {
            if task.parent_id.is_none() {
                root_tasks.push(task.id.clone());
            } else if let Some(parent_id) = &task.parent_id {
                if let Some(parent) = tasks.get_mut(parent_id) {
                    parent.children.push(task.id.clone());
                }
            }

            tasks.insert(task.id.clone(), task);
        }

        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    fn format_task(&self, tasks: &HashMap<String, Task>, task: &Task, indent: usize, output: &mut String) {
        let indent_str = "  ".repeat(indent);
        output.push_str(&format!(
            "{}- {} UUID:{} NAME:{} DESCRIPTION:{}\n",
            indent_str,
            task.state,
            task.id,
            task.name,
            task.description
        ));

        // Format children
        for child_id in &task.children {
            if let Some(child_task) = tasks.get(child_id) {
                self.format_task(tasks, child_task, indent + 1, output);
            }
        }
    }
}

// Global task list instance
lazy_static::lazy_static! {
    pub static ref GLOBAL_TASK_LIST: TaskList = TaskList::new();
}

/// Tool for viewing the current task list
pub struct ViewTasklistTool;

impl Default for ViewTasklistTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        let tasklist = GLOBAL_TASK_LIST.view_tasklist();
        Ok(ToolResult::success(&tool_call.id, self.name(), tasklist))
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

/// Tool for adding new tasks
pub struct AddTasksTool;

impl Default for AddTasksTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        "Add one or more new tasks to the task list. Can add a single task or multiple tasks in one call. Tasks can be added as subtasks or after specific tasks. Use this when planning complex sequences of work."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let tasks_value = tool_call.arguments
            .get("tasks")
            .ok_or_else(|| ToolError::InvalidArguments("Missing required parameter: tasks".to_string()))?;

        let tasks_array = tasks_value.as_array()
            .ok_or_else(|| ToolError::InvalidArguments("Tasks parameter must be an array".to_string()))?;

        let mut created_tasks = Vec::new();

        for task_value in tasks_array {
            let task_obj = task_value.as_object()
                .ok_or_else(|| ToolError::InvalidArguments("Each task must be an object".to_string()))?;

            let name = task_obj.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments("Task name is required".to_string()))?;

            let description = task_obj.get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments("Task description is required".to_string()))?;

            let parent_task_id = task_obj.get("parent_task_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let after_task_id = task_obj.get("after_task_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut task = Task::new(name.to_string(), description.to_string());

            if let Some(state_str) = task_obj.get("state").and_then(|v| v.as_str()) {
                task.state = match state_str {
                    "NOT_STARTED" => TaskState::NotStarted,
                    "IN_PROGRESS" => TaskState::InProgress,
                    "CANCELLED" => TaskState::Cancelled,
                    "COMPLETE" => TaskState::Complete,
                    _ => TaskState::NotStarted,
                };
            }

            let task_id = GLOBAL_TASK_LIST.add_task(task, parent_task_id, after_task_id)?;
            created_tasks.push(task_id);
        }

        Ok(ToolResult::success(&tool_call.id, self.name(), format!(
            "Successfully created {} task(s): {}",
            created_tasks.len(),
            created_tasks.join(", ")
        )))
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
                        "description": "Array of tasks to create. Each task should have name and description.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "The name of the new task."
                                },
                                "description": {
                                    "type": "string",
                                    "description": "The description of the new task."
                                },
                                "parent_task_id": {
                                    "type": "string",
                                    "description": "UUID of the parent task if this should be a subtask."
                                },
                                "after_task_id": {
                                    "type": "string",
                                    "description": "UUID of the task after which this task should be inserted."
                                },
                                "state": {
                                    "type": "string",
                                    "enum": ["NOT_STARTED", "IN_PROGRESS", "CANCELLED", "COMPLETE"],
                                    "description": "Initial state of the task. Defaults to NOT_STARTED."
                                }
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

/// Tool for updating existing tasks
pub struct UpdateTasksTool;

impl Default for UpdateTasksTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        "Update one or more tasks' properties (state, name, description). Can update a single task or multiple tasks in one call. Use this on complex sequences of work to plan, track progress, and manage work."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let tasks_value = tool_call.arguments
            .get("tasks")
            .ok_or_else(|| ToolError::InvalidArguments("Missing required parameter: tasks".to_string()))?;

        let tasks_array = tasks_value.as_array()
            .ok_or_else(|| ToolError::InvalidArguments("Tasks parameter must be an array".to_string()))?;

        let mut updated_tasks = Vec::new();
        let mut errors = Vec::new();

        for task_value in tasks_array {
            let task_obj = task_value.as_object()
                .ok_or_else(|| ToolError::InvalidArguments("Each task must be an object".to_string()))?;

            let task_id = task_obj.get("task_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments("Task task_id is required".to_string()))?;

            let name = task_obj.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let description = task_obj.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let state = task_obj.get("state")
                .and_then(|v| v.as_str())
                .map(|state_str| match state_str {
                    "NOT_STARTED" => TaskState::NotStarted,
                    "IN_PROGRESS" => TaskState::InProgress,
                    "CANCELLED" => TaskState::Cancelled,
                    "COMPLETE" => TaskState::Complete,
                    _ => TaskState::NotStarted,
                });

            match GLOBAL_TASK_LIST.update_task(task_id, name, description, state) {
                Ok(()) => updated_tasks.push(task_id.to_string()),
                Err(e) => errors.push(format!("Task {}: {}", task_id, e)),
            }
        }

        let mut result = format!("Task list updated successfully. Created: 0, Updated: {}, Deleted: 0.", updated_tasks.len());

        if !updated_tasks.is_empty() {
            result.push_str("\n\n# Task Changes\n\n## Updated Tasks\n\n");
            // Show current state of updated tasks - handle poison errors
            let tasks = match GLOBAL_TASK_LIST.tasks.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            for task_id in &updated_tasks {
                if let Some(task) = tasks.get(task_id) {
                    result.push_str(&format!(
                        "{} UUID:{} NAME:{} DESCRIPTION:{}\n",
                        task.state, task.id, task.name, task.description
                    ));
                }
            }
        }

        if !errors.is_empty() {
            result.push_str(&format!("\n\nErrors:\n{}", errors.join("\n")));
        }

        Ok(ToolResult::success(&tool_call.id, self.name(), result))
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
                        "description": "Array of tasks to update. Each task should have a task_id and the properties to update.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "task_id": {
                                    "type": "string",
                                    "description": "The UUID of the task to update."
                                },
                                "name": {
                                    "type": "string",
                                    "description": "New task name."
                                },
                                "description": {
                                    "type": "string",
                                    "description": "New task description."
                                },
                                "state": {
                                    "type": "string",
                                    "enum": ["NOT_STARTED", "IN_PROGRESS", "CANCELLED", "COMPLETE"],
                                    "description": "New task state. Use NOT_STARTED for [ ], IN_PROGRESS for [/], CANCELLED for [-], COMPLETE for [x]."
                                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sage_core::tools::types::ToolCall;
    use std::collections::HashMap;
    use serial_test::serial;

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

    // Helper function to clear the global task list safely
    // Handles poisoned mutex by recovering the inner data
    fn clear_global_task_list() {
        // Replace the global task list contents - handle poison errors
        match GLOBAL_TASK_LIST.tasks.lock() {
            Ok(mut tasks) => tasks.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        match GLOBAL_TASK_LIST.root_tasks.lock() {
            Ok(mut root_tasks) => root_tasks.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_view_empty_tasklist() {
        clear_global_task_list();

        let tool = ViewTasklistTool::new();
        let call = create_tool_call("test-1", "view_tasklist", json!({}));

        let result = tool.execute(&call).await.unwrap();
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("No tasks in the current task list"));
    }

    #[tokio::test]
    #[serial]
    async fn test_add_single_task() {
        clear_global_task_list();

        let tool = AddTasksTool::new();
        let call = create_tool_call("test-2", "add_tasks", json!({
            "tasks": [{
                "name": "Test Task",
                "description": "This is a test task",
                "state": "NOT_STARTED"
            }]
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Successfully created 1 task(s)"));

        // Verify task was added
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "view_tasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("Test Task"));
        assert!(view_result.output.as_ref().unwrap().contains("This is a test task"));
    }

    #[tokio::test]
    #[serial]
    async fn test_add_multiple_tasks() {
        clear_global_task_list();

        let tool = AddTasksTool::new();
        let call = create_tool_call("test-3", "add_tasks", json!({
            "tasks": [
                {
                    "name": "Task 1",
                    "description": "First task",
                    "state": "NOT_STARTED"
                },
                {
                    "name": "Task 2",
                    "description": "Second task",
                    "state": "IN_PROGRESS"
                }
            ]
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Successfully created 2 task(s)"));

        // Verify both tasks were added
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "view_tasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("Task 1"));
        assert!(view_result.output.as_ref().unwrap().contains("Task 2"));
        assert!(view_result.output.as_ref().unwrap().contains("[ ]")); // NOT_STARTED
        assert!(view_result.output.as_ref().unwrap().contains("[/]")); // IN_PROGRESS
    }

    #[tokio::test]
    #[serial]
    async fn test_update_task_state() {
        clear_global_task_list();

        // Add a task first
        let add_tool = AddTasksTool::new();
        let add_call = create_tool_call("test-add", "add_tasks", json!({
            "tasks": [{
                "name": "Update Test Task",
                "description": "Task to be updated",
                "state": "NOT_STARTED"
            }]
        }));
        add_tool.execute(&add_call).await.unwrap();

        // Get the task ID - handle potential poison errors
        let task_id = {
            let _tasks = match GLOBAL_TASK_LIST.tasks.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let root_tasks = match GLOBAL_TASK_LIST.root_tasks.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            root_tasks.first().expect("Task list should not be empty after adding a task").clone()
        };

        // Update the task
        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call("test-update", "update_tasks", json!({
            "tasks": [{
                "task_id": task_id,
                "state": "COMPLETE"
            }]
        }));

        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 1"));

        // Verify the update
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "view_tasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("[x]")); // COMPLETE
    }

    #[tokio::test]
    #[serial]
    async fn test_update_multiple_tasks() {
        clear_global_task_list();

        // Add multiple tasks
        let add_tool = AddTasksTool::new();
        let add_call = create_tool_call("test-add", "add_tasks", json!({
            "tasks": [
                {
                    "name": "Task A",
                    "description": "First task",
                    "state": "NOT_STARTED"
                },
                {
                    "name": "Task B",
                    "description": "Second task",
                    "state": "NOT_STARTED"
                }
            ]
        }));
        add_tool.execute(&add_call).await.unwrap();

        // Get task IDs - handle poison errors
        let (task_id_1, task_id_2) = {
            let root_tasks = match GLOBAL_TASK_LIST.root_tasks.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            (root_tasks[0].clone(), root_tasks[1].clone())
        };

        // Update both tasks
        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call("test-update", "update_tasks", json!({
            "tasks": [
                {
                    "task_id": task_id_1,
                    "state": "COMPLETE"
                },
                {
                    "task_id": task_id_2,
                    "state": "IN_PROGRESS"
                }
            ]
        }));

        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 2"));

        // Verify the updates
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "view_tasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("[x]")); // COMPLETE
        assert!(view_result.output.as_ref().unwrap().contains("[/]")); // IN_PROGRESS
    }

    #[tokio::test]
    #[serial]
    async fn test_update_nonexistent_task() {
        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call("test-update", "update_tasks", json!({
            "tasks": [{
                "task_id": "nonexistent-id",
                "state": "COMPLETE"
            }]
        }));

        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Errors:"));
        assert!(result.output.as_ref().unwrap().contains("Task not found"));
    }

    #[tokio::test]
    #[serial]
    async fn test_task_state_display() {
        assert_eq!(format!("{}", TaskState::NotStarted), "[ ]");
        assert_eq!(format!("{}", TaskState::InProgress), "[/]");
        assert_eq!(format!("{}", TaskState::Cancelled), "[-]");
        assert_eq!(format!("{}", TaskState::Complete), "[x]");
    }

    #[tokio::test]
    #[serial]
    async fn test_full_workflow_integration() {
        clear_global_task_list();

        // Step 1: View empty task list
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("view-1", "view_tasklist", json!({}));
        let result = view_tool.execute(&view_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("No tasks in the current task list"));

        // Step 2: Add some tasks
        let add_tool = AddTasksTool::new();
        let add_call = create_tool_call("add-1", "add_tasks", json!({
            "tasks": [
                {
                    "name": "Setup Project",
                    "description": "Initialize the project structure",
                    "state": "NOT_STARTED"
                },
                {
                    "name": "Implement Core Features",
                    "description": "Build the main functionality",
                    "state": "NOT_STARTED"
                },
                {
                    "name": "Write Tests",
                    "description": "Create comprehensive test suite",
                    "state": "NOT_STARTED"
                }
            ]
        }));
        let result = add_tool.execute(&add_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Successfully created 3 task(s)"));

        // Step 3: View the task list with tasks
        let view_call = create_tool_call("view-2", "view_tasklist", json!({}));
        let result = view_tool.execute(&view_call).await.unwrap();
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Setup Project"));
        assert!(output.contains("Implement Core Features"));
        assert!(output.contains("Write Tests"));
        assert!(output.contains("[ ]")); // All should be NOT_STARTED

        // Step 4: Start working on first task - handle poison errors
        let task_ids = {
            let root_tasks = match GLOBAL_TASK_LIST.root_tasks.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            root_tasks.clone()
        };

        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call("update-1", "update_tasks", json!({
            "tasks": [{
                "task_id": task_ids[0],
                "state": "IN_PROGRESS"
            }]
        }));
        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 1"));

        // Step 5: Complete first task and start second
        let update_call = create_tool_call("update-2", "update_tasks", json!({
            "tasks": [
                {
                    "task_id": task_ids[0],
                    "state": "COMPLETE"
                },
                {
                    "task_id": task_ids[1],
                    "state": "IN_PROGRESS"
                }
            ]
        }));
        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 2"));

        // Step 6: View final state
        let view_call = create_tool_call("view-3", "view_tasklist", json!({}));
        let result = view_tool.execute(&view_call).await.unwrap();
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("[x]")); // COMPLETE
        assert!(output.contains("[/]")); // IN_PROGRESS
        assert!(output.contains("[ ]")); // NOT_STARTED (third task)
    }
}
