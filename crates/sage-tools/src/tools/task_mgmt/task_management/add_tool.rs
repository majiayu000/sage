//! Tool for adding new tasks

use super::task_list::GLOBAL_TASK_LIST;
use super::types::{Task, TaskState};
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

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
        "AddTasks"
    }

    fn description(&self) -> &str {
        "Add one or more new tasks to the task list. Can add a single task or multiple tasks in one call. Tasks can be added as subtasks or after specific tasks. Use this when planning complex sequences of work."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let tasks_value = tool_call.arguments.get("tasks").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: tasks".to_string())
        })?;

        let tasks_array = tasks_value.as_array().ok_or_else(|| {
            ToolError::InvalidArguments("Tasks parameter must be an array".to_string())
        })?;

        let mut created_tasks = Vec::new();

        for task_value in tasks_array {
            let task_obj = task_value.as_object().ok_or_else(|| {
                ToolError::InvalidArguments("Each task must be an object".to_string())
            })?;

            let name = task_obj
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments("Task name is required".to_string()))?;

            let description = task_obj
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidArguments("Task description is required".to_string())
                })?;

            let parent_task_id = task_obj
                .get("parent_task_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let after_task_id = task_obj
                .get("after_task_id")
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

        Ok(ToolResult::success(
            &tool_call.id,
            self.name(),
            format!(
                "Successfully created {} task(s): {}",
                created_tasks.len(),
                created_tasks.join(", ")
            ),
        ))
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
