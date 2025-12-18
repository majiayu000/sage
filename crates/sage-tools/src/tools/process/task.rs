//! Task tool - Claude Code compatible subagent spawning
//!
//! Launches specialized sub-agents to handle complex tasks autonomously.

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

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
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(id.clone(), task);
        id
    }

    /// Get a task by ID
    pub fn get_task(&self, id: &str) -> Option<TaskRequest> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(id).cloned()
    }

    /// Update task status
    pub fn update_status(&self, id: &str, status: TaskStatus, result: Option<String>) {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = status;
            task.result = result;
        }
    }

    /// Get all pending tasks
    pub fn get_pending_tasks(&self) -> Vec<TaskRequest> {
        let tasks = self.tasks.read().unwrap();
        tasks.values()
            .filter(|t| t.status == TaskStatus::Pending)
            .cloned()
            .collect()
    }

    /// Get task result (blocks until complete or timeout)
    pub fn get_result(&self, id: &str) -> Option<String> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(id).and_then(|t| t.result.clone())
    }
}

// Global task registry
lazy_static::lazy_static! {
    pub static ref GLOBAL_TASK_REGISTRY: Arc<TaskRegistry> = Arc::new(TaskRegistry::new());
}

/// Task tool for spawning subagents
pub struct TaskTool {
    registry: Arc<TaskRegistry>,
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskTool {
    pub fn new() -> Self {
        Self {
            registry: GLOBAL_TASK_REGISTRY.clone(),
        }
    }

    pub fn with_registry(registry: Arc<TaskRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "Task"
    }

    fn description(&self) -> &str {
        r#"Launch a new agent to handle complex, multi-step tasks autonomously.

The Task tool launches specialized agents (subprocesses) that autonomously handle complex tasks. Each agent type has specific capabilities and tools available to it.

Available agent types:
- general-purpose: General-purpose agent with access to all tools. Use for complex multi-step tasks.
- Explore: Fast agent for codebase exploration. Use for finding files, searching code, or answering questions about the codebase. (Tools: Glob, Grep, Read, Bash)
- Plan: Software architect agent for designing implementation plans. Returns step-by-step plans and identifies critical files. (Tools: All)

When NOT to use the Task tool:
- If you want to read a specific file path, use Read or Glob instead
- If searching for a specific class definition, use Glob instead
- If searching code within 2-3 specific files, use Read instead

Usage notes:
- Launch multiple agents concurrently when possible (use single message with multiple tool calls)
- Agent results are not visible to the user - summarize results in your response
- Use run_in_background=true for background execution, then use TaskOutput to retrieve results
- Use resume parameter with agent ID to continue previous execution"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "A short (3-5 word) description of the task"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "The task for the agent to perform"
                    },
                    "subagent_type": {
                        "type": "string",
                        "description": "The type of specialized agent to use (general-purpose, Explore, Plan)"
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model to use (sonnet, opus, haiku). Defaults to inherit from parent.",
                        "enum": ["sonnet", "opus", "haiku"]
                    },
                    "run_in_background": {
                        "type": "boolean",
                        "description": "Set to true to run this agent in the background. Use TaskOutput to read output later.",
                        "default": false
                    },
                    "resume": {
                        "type": "string",
                        "description": "Optional agent ID to resume from. Agent continues with previous context preserved."
                    }
                },
                "required": ["description", "prompt", "subagent_type"]
            }),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Parse parameters
        let description = call.arguments.get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'description' parameter".to_string()))?
            .to_string();

        let prompt = call.arguments.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'prompt' parameter".to_string()))?
            .to_string();

        let subagent_type = call.arguments.get("subagent_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'subagent_type' parameter".to_string()))?
            .to_string();

        let model = call.arguments.get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let run_in_background = call.arguments.get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let resume = call.arguments.get("resume")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Generate task ID
        let task_id = resume.clone().unwrap_or_else(|| format!("task_{}", Uuid::new_v4()));

        // Create task request
        let task = TaskRequest {
            id: task_id.clone(),
            description: description.clone(),
            prompt: prompt.clone(),
            subagent_type: subagent_type.clone(),
            model,
            run_in_background,
            resume,
            status: TaskStatus::Pending,
            result: None,
        };

        // Register the task
        self.registry.add_task(task);

        // For now, return a placeholder response
        // In a full implementation, this would trigger the SubAgentExecutor
        // through a message channel to the main agent loop
        let response = if run_in_background {
            format!(
                "Task '{}' ({}) queued for background execution.\n\
                 Agent type: {}\n\
                 Task ID: {}\n\n\
                 Use TaskOutput with task_id=\"{}\" to retrieve results when ready.",
                description, task_id, subagent_type, task_id, task_id
            )
        } else {
            // Synchronous execution would happen here in a full implementation
            // For now, simulate an immediate response
            format!(
                "Task '{}' submitted to {} agent.\n\
                 Task ID: {}\n\n\
                 [Note: Full subagent execution requires integration with agent loop. \
                 Task has been registered and can be processed by the agent coordinator.]",
                description, subagent_type, task_id
            )
        };

        Ok(ToolResult {
            call_id: call.id.clone(),
            tool_name: self.name().to_string(),
            success: true,
            output: Some(response),
            error: None,
            exit_code: None,
            execution_time_ms: None,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("task_id".to_string(), json!(task_id));
                meta.insert("subagent_type".to_string(), json!(subagent_type));
                meta.insert("run_in_background".to_string(), json!(run_in_background));
                meta
            },
        })
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_tool_basic() {
        let registry = Arc::new(TaskRegistry::new());
        let tool = TaskTool::with_registry(registry.clone());

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Search codebase",
                "prompt": "Find all files related to authentication",
                "subagent_type": "Explore"
            }).as_object().unwrap().clone().into_iter().map(|(k, v)| (k, v)).collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Explore"));

        // Verify task was registered
        let tasks = registry.get_pending_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].subagent_type, "Explore");
    }

    #[tokio::test]
    async fn test_task_tool_background() {
        let registry = Arc::new(TaskRegistry::new());
        let tool = TaskTool::with_registry(registry.clone());

        let call = ToolCall {
            id: "test-2".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Plan implementation",
                "prompt": "Design authentication system",
                "subagent_type": "Plan",
                "run_in_background": true,
                "model": "opus"
            }).as_object().unwrap().clone().into_iter().map(|(k, v)| (k, v)).collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("background"));
        assert!(output.contains("TaskOutput"));
    }

    #[tokio::test]
    async fn test_task_registry() {
        let registry = TaskRegistry::new();

        let task = TaskRequest {
            id: "task-1".to_string(),
            description: "Test task".to_string(),
            prompt: "Do something".to_string(),
            subagent_type: "Explore".to_string(),
            model: None,
            run_in_background: false,
            resume: None,
            status: TaskStatus::Pending,
            result: None,
        };

        registry.add_task(task);

        // Get task
        let retrieved = registry.get_task("task-1").unwrap();
        assert_eq!(retrieved.description, "Test task");
        assert_eq!(retrieved.status, TaskStatus::Pending);

        // Update status
        registry.update_status("task-1", TaskStatus::Completed, Some("Done!".to_string()));

        let updated = registry.get_task("task-1").unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.result, Some("Done!".to_string()));

        // Pending tasks should be empty now
        let pending = registry.get_pending_tasks();
        assert_eq!(pending.len(), 0);
    }
}
