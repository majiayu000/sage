//! Agent execution representation

use crate::agent::AgentStep;
use crate::types::{Id, LLMUsage, TaskMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete execution of an agent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    /// Unique identifier for this execution
    pub id: Id,
    /// Task metadata
    pub task: TaskMetadata,
    /// All steps in the execution
    pub steps: Vec<AgentStep>,
    /// Final result of the execution
    pub final_result: Option<String>,
    /// Whether the execution was successful
    pub success: bool,
    /// Total token usage across all steps
    pub total_usage: LLMUsage,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Execution end time
    pub completed_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    // TODO: Add error recovery mechanism
    // - Track failed steps and retry attempts
    // - Implement intelligent retry strategies
    // - Add rollback capabilities for failed operations

    // TODO: Add execution checkpoints
    // - Support for pausing and resuming execution
    // - Implement state serialization for persistence
    // - Add checkpoint-based recovery

    // TODO: Add execution metrics
    // - Track performance metrics per step
    // - Monitor resource usage during execution
    // - Generate execution analytics and insights
}

impl AgentExecution {
    /// Create a new agent execution
    pub fn new(task: TaskMetadata) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            task,
            steps: Vec::new(),
            final_result: None,
            success: false,
            total_usage: LLMUsage::default(),
            started_at: Utc::now(),
            completed_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a step to the execution
    pub fn add_step(&mut self, step: AgentStep) {
        // Update total usage
        if let Some(usage) = &step.llm_usage {
            self.total_usage.add(usage);
        }

        self.steps.push(step);
    }

    /// Mark the execution as completed
    pub fn complete(&mut self, success: bool, final_result: Option<String>) {
        self.success = success;
        self.final_result = final_result;
        self.completed_at = Some(Utc::now());
        self.task.complete();
    }

    /// Get the current step number
    pub fn current_step_number(&self) -> u32 {
        self.steps.len() as u32 + 1
    }

    /// Get the last step
    pub fn last_step(&self) -> Option<&AgentStep> {
        self.steps.last()
    }

    /// Get the last step mutably
    pub fn last_step_mut(&mut self) -> Option<&mut AgentStep> {
        self.steps.last_mut()
    }

    /// Get execution duration
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed - self.started_at)
    }

    /// Check if the execution is completed
    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Get a summary of the execution
    pub fn summary(&self) -> String {
        let status = if self.success { "SUCCESS" } else { "FAILED" };
        let duration = self
            .duration()
            .map(|d| format!(" in {:.2}s", d.num_milliseconds() as f64 / 1000.0))
            .unwrap_or_default();

        // Build cache info if available
        let cache_info = if self.total_usage.has_cache_metrics() {
            let mut parts = Vec::new();
            if let Some(created) = self.total_usage.cache_creation_input_tokens {
                if created > 0 {
                    parts.push(format!("{} cache created", created));
                }
            }
            if let Some(read) = self.total_usage.cache_read_input_tokens {
                if read > 0 {
                    parts.push(format!("{} cache read", read));
                }
            }
            if !parts.is_empty() {
                format!(", {}", parts.join(", "))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!(
            "Execution {}: {} ({} steps, {} tokens{}{})",
            status,
            self.task.description,
            self.steps.len(),
            self.total_usage.total_tokens,
            cache_info,
            duration
        )
    }

    /// Get steps that had errors
    pub fn error_steps(&self) -> Vec<&AgentStep> {
        self.steps
            .iter()
            .filter(|step| step.error.is_some())
            .collect()
    }

    /// Get steps that made tool calls
    pub fn tool_steps(&self) -> Vec<&AgentStep> {
        self.steps
            .iter()
            .filter(|step| step.has_tool_calls())
            .collect()
    }

    /// Get all tool calls made during execution
    pub fn all_tool_calls(&self) -> Vec<&crate::tools::ToolCall> {
        self.steps
            .iter()
            .flat_map(|step| &step.tool_calls)
            .collect()
    }

    /// Get all tool results from execution
    pub fn all_tool_results(&self) -> Vec<&crate::tools::ToolResult> {
        self.steps
            .iter()
            .flat_map(|step| &step.tool_results)
            .collect()
    }

    /// Check if any step indicates task completion
    pub fn indicates_completion(&self) -> bool {
        self.steps.iter().any(|step| step.indicates_completion())
    }

    /// Add metadata to the execution
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get execution statistics
    pub fn statistics(&self) -> ExecutionStatistics {
        let mut stats = ExecutionStatistics::default();

        stats.total_steps = self.steps.len();
        stats.successful_steps = self.steps.iter().filter(|s| s.error.is_none()).count();
        stats.failed_steps = self.steps.iter().filter(|s| s.error.is_some()).count();
        stats.tool_calls = self.all_tool_calls().len();
        stats.total_tokens = self.total_usage.total_tokens;
        stats.cache_creation_tokens = self.total_usage.cache_creation_input_tokens;
        stats.cache_read_tokens = self.total_usage.cache_read_input_tokens;
        stats.execution_time = self.duration();

        // Count tool usage
        for step in &self.steps {
            for tool_call in &step.tool_calls {
                *stats.tool_usage.entry(tool_call.name.clone()).or_insert(0) += 1;
            }
        }

        stats
    }
}

/// Statistics about an execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Total number of steps
    pub total_steps: usize,
    /// Number of successful steps
    pub successful_steps: usize,
    /// Number of failed steps
    pub failed_steps: usize,
    /// Total number of tool calls
    pub tool_calls: usize,
    /// Total tokens used
    pub total_tokens: u32,
    /// Tokens written to cache (Anthropic prompt caching)
    pub cache_creation_tokens: Option<u32>,
    /// Tokens read from cache (Anthropic prompt caching)
    pub cache_read_tokens: Option<u32>,
    /// Execution time
    pub execution_time: Option<chrono::Duration>,
    /// Tool usage count by tool name
    pub tool_usage: HashMap<String, usize>,
}
