//! Agent execution representation

use crate::agent::AgentStep;
use crate::types::{Id, LlmUsage, TaskMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete execution of an agent task
///
/// # Future Enhancements
///
/// The following features are planned for future versions:
///
/// - **Error Recovery**: Track failed steps and retry attempts, intelligent retry
///   strategies, rollback capabilities for failed operations
/// - **Execution Checkpoints**: Pause/resume support, state serialization for
///   persistence, checkpoint-based recovery
/// - **Execution Metrics**: Performance metrics per step, resource usage monitoring,
///   execution analytics and insights
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
    pub total_usage: LlmUsage,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Execution end time
    pub completed_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
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
            total_usage: LlmUsage::default(),
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
            if let Some(created) = self
                .total_usage
                .cache_creation_input_tokens
                .filter(|&c| c > 0)
            {
                parts.push(format!("{} cache created", created));
            }
            if let Some(read) = self.total_usage.cache_read_input_tokens.filter(|&r| r > 0) {
                parts.push(format!("{} cache read", read));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentState;
    use crate::types::LlmUsage;

    fn create_test_task() -> TaskMetadata {
        TaskMetadata::new("Test task", ".")
    }

    #[test]
    fn test_new_execution() {
        let task = create_test_task();
        let execution = AgentExecution::new(task.clone());

        assert_eq!(execution.task.description, "Test task");
        assert_eq!(execution.steps.len(), 0);
        assert!(!execution.success);
        assert!(execution.final_result.is_none());
        assert!(execution.completed_at.is_none());
        assert_eq!(execution.total_usage.total_tokens, 0);
    }

    #[test]
    fn test_add_step() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        let step = AgentStep::new(1, AgentState::Thinking);
        execution.add_step(step);

        assert_eq!(execution.steps.len(), 1);
        assert_eq!(execution.current_step_number(), 2);
    }

    #[test]
    fn test_add_step_with_usage() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        let mut step = AgentStep::new(1, AgentState::Thinking);
        step.llm_usage = Some(LlmUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            cost_usd: None,
        });

        execution.add_step(step);

        assert_eq!(execution.total_usage.total_tokens, 150);
        assert_eq!(execution.total_usage.prompt_tokens, 100);
        assert_eq!(execution.total_usage.completion_tokens, 50);
    }

    #[test]
    fn test_complete_execution() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        execution.complete(true, Some("Task completed successfully".to_string()));

        assert!(execution.success);
        assert_eq!(
            execution.final_result,
            Some("Task completed successfully".to_string())
        );
        assert!(execution.completed_at.is_some());
        assert!(execution.is_completed());
    }

    #[test]
    fn test_last_step() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        assert!(execution.last_step().is_none());

        let step1 = AgentStep::new(1, AgentState::Thinking);
        execution.add_step(step1);

        let step2 = AgentStep::new(2, AgentState::ToolExecution);
        execution.add_step(step2);

        let last = execution.last_step();
        assert!(last.is_some());
        assert_eq!(last.unwrap().step_number, 2);
    }

    #[test]
    fn test_duration() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        // Duration is None before completion
        assert!(execution.duration().is_none());

        execution.complete(true, None);

        // Duration should be Some after completion
        let duration = execution.duration();
        assert!(duration.is_some());
        assert!(duration.unwrap().num_milliseconds() >= 0);
    }

    #[test]
    fn test_summary() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        let mut step = AgentStep::new(1, AgentState::Thinking);
        step.llm_usage = Some(LlmUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            cost_usd: None,
        });
        execution.add_step(step);

        execution.complete(true, None);

        let summary = execution.summary();
        assert!(summary.contains("SUCCESS"));
        assert!(summary.contains("Test task"));
        assert!(summary.contains("1 steps"));
        assert!(summary.contains("150 tokens"));
    }

    #[test]
    fn test_with_metadata() {
        let task = create_test_task();
        let execution = AgentExecution::new(task)
            .with_metadata("key1", "value1")
            .with_metadata("key2", 42);

        assert_eq!(execution.metadata.len(), 2);
        assert_eq!(
            execution.metadata.get("key1").unwrap().as_str().unwrap(),
            "value1"
        );
        assert_eq!(
            execution.metadata.get("key2").unwrap().as_i64().unwrap(),
            42
        );
    }

    #[test]
    fn test_statistics() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        let mut step1 = AgentStep::new(1, AgentState::Thinking);
        step1.llm_usage = Some(LlmUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cache_creation_input_tokens: Some(50),
            cache_read_input_tokens: Some(25),
            cost_usd: None,
        });
        execution.add_step(step1);

        let mut step2 = AgentStep::new(2, AgentState::Error);
        step2.error = Some("Test error".to_string());
        execution.add_step(step2);

        execution.complete(false, None);

        let stats = execution.statistics();
        assert_eq!(stats.total_steps, 2);
        assert_eq!(stats.failed_steps, 1);
        assert_eq!(stats.successful_steps, 1);
        assert_eq!(stats.total_tokens, 150);
        assert_eq!(stats.cache_creation_tokens, Some(50));
        assert_eq!(stats.cache_read_tokens, Some(25));
    }

    #[test]
    fn test_current_step_number() {
        let task = create_test_task();
        let mut execution = AgentExecution::new(task);

        assert_eq!(execution.current_step_number(), 1);

        execution.add_step(AgentStep::new(1, AgentState::Thinking));
        assert_eq!(execution.current_step_number(), 2);

        execution.add_step(AgentStep::new(2, AgentState::Thinking));
        assert_eq!(execution.current_step_number(), 3);
    }
}
