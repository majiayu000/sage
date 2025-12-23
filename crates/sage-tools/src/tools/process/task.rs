//! Task tool - Claude Code compatible subagent spawning
//!
//! Launches specialized sub-agents to handle complex tasks autonomously.
//! Now with actual execution support via SubAgentRunner.

use async_trait::async_trait;
use parking_lot::RwLock;
use sage_core::agent::subagent::{AgentType, SubAgentConfig, Thoroughness, execute_subagent};
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
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
- Explore: Fast agent for codebase exploration. Use for finding files, searching code, or answering questions about the codebase. (Tools: Glob, Grep, Read, Bash). Supports thoroughness levels: "quick", "medium", "very_thorough".
- Plan: Software architect agent for designing implementation plans. Returns step-by-step plans and identifies critical files. (Tools: All)

When NOT to use the Task tool:
- If you want to read a specific file path, use Read or Glob instead
- If searching for a specific class definition, use Glob instead
- If searching code within 2-3 specific files, use Read instead

Usage notes:
- Launch multiple agents concurrently when possible (use single message with multiple tool calls)
- Agent results are not visible to the user - summarize results in your response
- Use run_in_background=true for background execution, then use TaskOutput to retrieve results
- Use resume parameter with agent ID to continue previous execution
- For Explore agents, specify thoroughness: "quick" (5 steps), "medium" (15 steps), or "very_thorough" (30 steps)"#
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
                    },
                    "thoroughness": {
                        "type": "string",
                        "description": "Thoroughness level for Explore agents: quick (fast, 5 steps), medium (balanced, 15 steps), very_thorough (comprehensive, 30 steps). Default: medium.",
                        "enum": ["quick", "medium", "very_thorough"],
                        "default": "medium"
                    }
                },
                "required": ["description", "prompt", "subagent_type"]
            }),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Parse parameters
        let description = call
            .arguments
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments("Missing 'description' parameter".to_string())
            })?
            .to_string();

        let prompt = call
            .arguments
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'prompt' parameter".to_string()))?
            .to_string();

        let subagent_type_str = call
            .arguments
            .get("subagent_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments("Missing 'subagent_type' parameter".to_string())
            })?
            .to_string();

        let model = call
            .arguments
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let run_in_background = call
            .arguments
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let resume = call
            .arguments
            .get("resume")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse thoroughness level
        let thoroughness = call
            .arguments
            .get("thoroughness")
            .and_then(|v| v.as_str())
            .map(|s| match s.to_lowercase().as_str() {
                "quick" => Thoroughness::Quick,
                "very_thorough" | "very-thorough" | "thorough" => Thoroughness::VeryThorough,
                _ => Thoroughness::Medium,
            })
            .unwrap_or(Thoroughness::Medium);

        // Generate task ID
        let task_id = resume
            .clone()
            .unwrap_or_else(|| format!("task_{}", Uuid::new_v4()));

        // Parse agent type
        let agent_type = match subagent_type_str.to_lowercase().as_str() {
            "explore" => AgentType::Explore,
            "plan" => AgentType::Plan,
            "general-purpose" | "general_purpose" | "general" => AgentType::GeneralPurpose,
            _ => AgentType::GeneralPurpose, // Default to general purpose
        };

        // Create task request for tracking
        let task = TaskRequest {
            id: task_id.clone(),
            description: description.clone(),
            prompt: prompt.clone(),
            subagent_type: subagent_type_str.clone(),
            model: model.clone(),
            run_in_background,
            resume,
            status: TaskStatus::Running,
            result: None,
        };

        // Register the task
        self.registry.add_task(task);

        // Handle background vs synchronous execution
        if run_in_background {
            // Background execution - return immediately, task will be processed async
            let response = format!(
                "Task '{}' ({}) queued for background execution.\n\
                 Agent type: {}\n\
                 Task ID: {}\n\n\
                 Use TaskOutput with task_id=\"{}\" to retrieve results when ready.",
                description, task_id, subagent_type_str, task_id, task_id
            );

            // TODO: Spawn background task here
            // For now, update status to pending for background processing
            self.registry
                .update_status(&task_id, TaskStatus::Pending, None);

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
                    meta.insert("subagent_type".to_string(), json!(subagent_type_str));
                    meta.insert("run_in_background".to_string(), json!(true));
                    meta
                },
            })
        } else {
            // Synchronous execution - actually run the subagent
            let config =
                SubAgentConfig::new(agent_type, prompt.clone()).with_thoroughness(thoroughness);

            match execute_subagent(config).await {
                Ok(result) => {
                    // Update task status
                    self.registry.update_status(
                        &task_id,
                        TaskStatus::Completed,
                        Some(result.content.clone()),
                    );

                    let response = format!(
                        "## Sub-agent Result ({})\n\n\
                         **Agent ID**: {}\n\
                         **Execution Time**: {}ms\n\
                         **Tools Used**: {}\n\
                         **Total Tool Calls**: {}\n\n\
                         ---\n\n\
                         {}",
                        subagent_type_str,
                        result.agent_id,
                        result.metadata.execution_time_ms,
                        result.metadata.tools_used.join(", "),
                        result.metadata.total_tool_uses,
                        result.content
                    );

                    Ok(ToolResult {
                        call_id: call.id.clone(),
                        tool_name: self.name().to_string(),
                        success: true,
                        output: Some(response),
                        error: None,
                        exit_code: None,
                        execution_time_ms: Some(result.metadata.execution_time_ms),
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("task_id".to_string(), json!(task_id));
                            meta.insert("agent_id".to_string(), json!(result.agent_id));
                            meta.insert("subagent_type".to_string(), json!(subagent_type_str));
                            meta.insert(
                                "tools_used".to_string(),
                                json!(result.metadata.tools_used),
                            );
                            meta.insert(
                                "total_tool_uses".to_string(),
                                json!(result.metadata.total_tool_uses),
                            );
                            meta
                        },
                    })
                }
                Err(e) => {
                    // Update task status to failed
                    self.registry
                        .update_status(&task_id, TaskStatus::Failed, Some(e.to_string()));

                    // Check if runner is not initialized
                    let error_msg = if e.to_string().contains("not initialized") {
                        format!(
                            "Sub-agent execution failed: Runner not initialized.\n\n\
                             This usually means the agent was started without sub-agent support.\n\
                             Error: {}",
                            e
                        )
                    } else {
                        format!("Sub-agent execution failed: {}", e)
                    };

                    Ok(ToolResult {
                        call_id: call.id.clone(),
                        tool_name: self.name().to_string(),
                        success: false,
                        output: None,
                        error: Some(error_msg),
                        exit_code: Some(1),
                        execution_time_ms: None,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("task_id".to_string(), json!(task_id));
                            meta.insert("subagent_type".to_string(), json!(subagent_type_str));
                            meta
                        },
                    })
                }
            }
        }
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

        // When global runner is not initialized, the task will fail with a helpful message
        // This is expected in test environments without full agent setup
        if !result.success {
            let error = result.error.unwrap();
            assert!(
                error.contains("not initialized") || error.contains("Runner not initialized"),
                "Expected 'not initialized' error, got: {}",
                error
            );

            // Get task_id from metadata
            if let Some(task_id) = result.metadata.get("task_id").and_then(|v| v.as_str()) {
                // Verify task was registered and marked as failed
                let task = registry.get_task(task_id);
                assert!(
                    task.map(|t| t.status == TaskStatus::Failed)
                        .unwrap_or(false)
                );
            }
        } else {
            // If runner is initialized, verify successful execution
            assert!(result.output.unwrap().contains("Explore"));
        }
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
