//! Task types and registry for subagent management

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Task request for subagent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    /// Unique task ID
    pub id: String,
    /// Short description (3-5 words)
    pub description: String,
    /// Detailed prompt for the agent
    pub prompt: String,
    /// Subagent type (Explore, Plan, general-purpose, etc.)
    pub subagent_type: String,
    /// Optional model override (sonnet, opus, haiku)
    pub model: Option<String>,
    /// Whether to run in background
    pub run_in_background: bool,
    /// Optional agent ID to resume from
    pub resume: Option<String>,
    /// Status of the task
    pub status: TaskStatus,
    /// Result content (when completed)
    pub result: Option<String>,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Global task registry for tracking spawned tasks
#[derive(Debug, Default)]
pub struct TaskRegistry {
    tasks: RwLock<HashMap<String, TaskRequest>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
        }
    }

    /// Add a new task
    pub fn add_task(&self, task: TaskRequest) -> String {
        let id = task.id.clone();
        let mut tasks = self.tasks.write();
        tasks.insert(id.clone(), task);
        id
    }

    /// Get a task by ID
    pub fn get_task(&self, id: &str) -> Option<TaskRequest> {
        let tasks = self.tasks.read();
        tasks.get(id).cloned()
    }

    /// Update task status
    pub fn update_status(&self, id: &str, status: TaskStatus, result: Option<String>) {
        let mut tasks = self.tasks.write();
        if let Some(task) = tasks.get_mut(id) {
            task.status = status;
            task.result = result;
        }
    }

    /// Get all pending tasks
    pub fn get_pending_tasks(&self) -> Vec<TaskRequest> {
        let tasks = self.tasks.read();
        tasks
            .values()
            .filter(|t| t.status == TaskStatus::Pending)
            .cloned()
            .collect()
    }

    /// Get task result (blocks until complete or timeout)
    pub fn get_result(&self, id: &str) -> Option<String> {
        let tasks = self.tasks.read();
        tasks.get(id).and_then(|t| t.result.clone())
    }
}

// Global task registry
pub static GLOBAL_TASK_REGISTRY: Lazy<Arc<TaskRegistry>> =
    Lazy::new(|| Arc::new(TaskRegistry::new()));

/// Get pending tasks from the global registry
pub fn get_pending_tasks() -> Vec<TaskRequest> {
    GLOBAL_TASK_REGISTRY.get_pending_tasks()
}

/// Update a task's status
pub fn update_task_status(task_id: &str, status: TaskStatus, result: Option<String>) {
    GLOBAL_TASK_REGISTRY.update_status(task_id, status, result);
}

/// Get a task by ID
pub fn get_task(task_id: &str) -> Option<TaskRequest> {
    GLOBAL_TASK_REGISTRY.get_task(task_id)
}
