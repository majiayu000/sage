//! Agent step representation

use crate::agent::state::AgentState;
use crate::llm::LlmResponse;
use crate::tools::{ToolCall, ToolResult};
use crate::types::{Id, LlmUsage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single step in an agent's execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Unique identifier for this step
    pub id: Id,
    /// Step number in the execution sequence
    pub step_number: u32,
    /// Current state of the agent
    pub state: AgentState,
    /// Timestamp when the step started
    pub started_at: DateTime<Utc>,
    /// Timestamp when the step completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Agent's thought process (if any)
    pub thought: Option<String>,
    /// Input messages sent to LLM
    pub llm_messages: Option<Vec<serde_json::Value>>,
    /// Tool calls made in this step
    pub tool_calls: Vec<ToolCall>,
    /// Results from tool executions
    pub tool_results: Vec<ToolResult>,
    /// LLM response for this step
    pub llm_response: Option<LlmResponse>,
    /// Reflection or analysis of the step
    pub reflection: Option<String>,
    /// Error message if the step failed
    pub error: Option<String>,
    /// Token usage for this step
    pub llm_usage: Option<LlmUsage>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentStep {
    /// Create a new agent step
    pub fn new(step_number: u32, state: AgentState) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            step_number,
            state,
            started_at: Utc::now(),
            completed_at: None,
            thought: None,
            llm_messages: None,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            llm_response: None,
            reflection: None,
            error: None,
            llm_usage: None,
            metadata: HashMap::new(),
        }
    }

    /// Mark the step as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Set the agent's thought for this step
    pub fn with_thought<S: Into<String>>(mut self, thought: S) -> Self {
        self.thought = Some(thought.into());
        self
    }

    /// Add input messages sent to LLM
    pub fn with_llm_messages(mut self, messages: Vec<serde_json::Value>) -> Self {
        self.llm_messages = Some(messages);
        self
    }

    /// Add an LLM response to this step
    pub fn with_llm_response(mut self, response: LlmResponse) -> Self {
        // Extract tool calls from the response
        self.tool_calls = response.tool_calls.clone();

        // Extract usage information
        if let Some(usage) = &response.usage {
            self.llm_usage = Some(usage.clone());
        }

        self.llm_response = Some(response);
        self
    }

    /// Add tool results to this step
    pub fn with_tool_results(mut self, results: Vec<ToolResult>) -> Self {
        self.tool_results = results;
        self
    }

    /// Set an error for this step
    pub fn with_error<S: Into<String>>(mut self, error: S) -> Self {
        self.error = Some(error.into());
        self.state = AgentState::Error;
        self
    }

    /// Add metadata to this step
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the duration of this step
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed - self.started_at)
    }

    /// Check if this step has tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Check if this step has tool results
    pub fn has_tool_results(&self) -> bool {
        !self.tool_results.is_empty()
    }

    /// Check if all tool calls have corresponding results
    pub fn tool_calls_completed(&self) -> bool {
        if self.tool_calls.is_empty() {
            return true;
        }

        let call_ids: std::collections::HashSet<_> =
            self.tool_calls.iter().map(|call| &call.id).collect();
        let result_ids: std::collections::HashSet<_> = self
            .tool_results
            .iter()
            .map(|result| &result.call_id)
            .collect();

        call_ids == result_ids
    }

    /// Check if this step indicates task completion
    pub fn indicates_completion(&self) -> bool {
        self.tool_calls.iter().any(|call| call.name == "task_done")
            || self.state == AgentState::Completed
    }

    /// Get a summary of this step
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("Step {}: {}", self.step_number, self.state));

        if let Some(thought) = &self.thought {
            parts.push(format!(
                "Thought: {}",
                thought.chars().take(100).collect::<String>()
            ));
        }

        if !self.tool_calls.is_empty() {
            let tool_names: Vec<_> = self
                .tool_calls
                .iter()
                .map(|call| call.name.as_str())
                .collect();
            parts.push(format!("Tools: {}", tool_names.join(", ")));
        }

        if let Some(error) = &self.error {
            parts.push(format!("Error: {}", error));
        }

        parts.join(" | ")
    }
}
