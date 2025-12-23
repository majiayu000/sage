//! Task list management and operations

use super::types::{Task, TaskState};
use parking_lot::Mutex;
use sage_core::tools::base::ToolError;
use std::collections::HashMap;
use std::sync::Arc;

/// Task list manager
#[derive(Debug, Clone)]
pub struct TaskList {
    pub tasks: Arc<Mutex<HashMap<String, Task>>>,
    pub root_tasks: Arc<Mutex<Vec<String>>>,
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

    pub fn add_task(
        &self,
        mut task: Task,
        parent_id: Option<String>,
        after_task_id: Option<String>,
    ) -> Result<String, ToolError> {
        let mut tasks = self.tasks.lock();
        let mut root_tasks = self.root_tasks.lock();

        task.parent_id = parent_id.clone();
        let task_id = task.id.clone();

        if let Some(parent_id) = &parent_id {
            // Add as subtask
            if let Some(parent) = tasks.get_mut(parent_id) {
                parent.children.push(task_id.clone());
            } else {
                return Err(ToolError::InvalidArguments(format!(
                    "Parent task not found: {}",
                    parent_id
                )));
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

    pub fn update_task(
        &self,
        task_id: &str,
        name: Option<String>,
        description: Option<String>,
        state: Option<TaskState>,
    ) -> Result<(), ToolError> {
        let mut tasks = self.tasks.lock();

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
            Err(ToolError::InvalidArguments(format!(
                "Task not found: {}",
                task_id
            )))
        }
    }

    pub fn view_tasklist(&self) -> String {
        let tasks = self.tasks.lock();
        let root_tasks = self.root_tasks.lock();

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
        let root_tasks = self.root_tasks.lock();
        root_tasks.clone()
    }

    pub fn clear_and_rebuild(&self, new_tasks: Vec<Task>) -> Result<(), ToolError> {
        let mut tasks = self.tasks.lock();
        let mut root_tasks = self.root_tasks.lock();

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
    fn format_task(
        &self,
        tasks: &HashMap<String, Task>,
        task: &Task,
        indent: usize,
        output: &mut String,
    ) {
        let indent_str = "  ".repeat(indent);
        output.push_str(&format!(
            "{}- {} UUID:{} NAME:{} DESCRIPTION:{}\n",
            indent_str, task.state, task.id, task.name, task.description
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
