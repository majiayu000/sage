//! Progress tracking and execution metadata

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Progress information for running agent
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProgress {
    /// Recent activity descriptions
    pub recent_activities: VecDeque<String>,
    /// Total tokens consumed so far
    pub token_count: u64,
    /// Number of tools used
    pub tool_use_count: u32,
    /// Current execution step
    pub current_step: u32,
}

impl fmt::Display for AgentProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Progress(step: {}, tools: {}, tokens: {})",
            self.current_step, self.tool_use_count, self.token_count
        )
    }
}

impl AgentProgress {
    /// Create new progress
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new activity to the progress tracker
    pub fn add_activity(&mut self, activity: String) {
        self.recent_activities.push_back(activity);
        // Keep only the last 10 activities
        if self.recent_activities.len() > 10 {
            self.recent_activities.pop_front();
        }
    }

    /// Increment tool use counter
    pub fn increment_tool_use(&mut self) {
        self.tool_use_count += 1;
    }

    /// Add tokens to the counter
    pub fn add_tokens(&mut self, tokens: u64) {
        self.token_count += tokens;
    }

    /// Advance to next step
    pub fn next_step(&mut self) {
        self.current_step += 1;
    }
}

/// Execution metadata collected during agent run
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionMetadata {
    /// Total tokens consumed
    pub total_tokens: u64,
    /// Total number of tool uses
    pub total_tool_uses: u32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// List of tools used during execution
    pub tools_used: Vec<String>,
}

impl fmt::Display for ExecutionMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Metadata(tokens: {}, tools: {}, time: {}ms)",
            self.total_tokens, self.total_tool_uses, self.execution_time_ms
        )
    }
}

impl ExecutionMetadata {
    /// Create new metadata from agent progress
    pub fn from_progress(progress: &AgentProgress, elapsed_ms: u64) -> Self {
        Self {
            total_tokens: progress.token_count,
            total_tool_uses: progress.tool_use_count,
            execution_time_ms: elapsed_ms,
            tools_used: Vec::new(),
        }
    }

    /// Add a tool to the tools_used list (deduplicates)
    pub fn add_tool(&mut self, tool_name: String) {
        if !self.tools_used.contains(&tool_name) {
            self.tools_used.push(tool_name);
        }
    }
}
