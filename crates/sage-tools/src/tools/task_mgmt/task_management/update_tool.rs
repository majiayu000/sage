//! Tool for updating existing tasks

use super::task_list::GLOBAL_TASK_LIST;
use super::types::TaskState;
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

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
        "UpdateTasks"
    }

    fn description(&self) -> &str {
        "Update one or more tasks' properties (state, name, description). Can update a single task or multiple tasks in one call. Use this on complex sequences of work to plan, track progress, and manage work."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let tasks_value = tool_call.arguments.get("tasks").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: tasks".to_string())
        })?;

        let tasks_array = tasks_value.as_array().ok_or_else(|| {
            ToolError::InvalidArguments("Tasks parameter must be an array".to_string())
        })?;

        let mut updated_tasks = Vec::new();
        let mut errors = Vec::new();

        for task_value in tasks_array {
            let task_obj = task_value.as_object().ok_or_else(|| {
                ToolError::InvalidArguments("Each task must be an object".to_string())
            })?;

            let task_id = task_obj
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidArguments("Task task_id is required".to_string())
                })?;

            let name = task_obj
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let description = task_obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let state = task_obj
                .get("state")
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

        let mut result = format!(
            "Task list updated successfully. Created: 0, Updated: {}, Deleted: 0.",
            updated_tasks.len()
        );

        if !updated_tasks.is_empty() {
            result.push_str("\n\n# Task Changes\n\n## Updated Tasks\n\n");
            // Show current state of updated tasks - handle poison errors
            let tasks = GLOBAL_TASK_LIST.tasks.lock();
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
