//! Types and data structures for reactive agent execution

use crate::tools::types::{ToolCall, ToolResult};
use crate::types::Id;
use std::sync::atomic::{AtomicU64, Ordering};

/// Token usage tracking across all steps
#[derive(Debug, Default)]
pub struct TokenUsage {
    /// Total input tokens consumed
    pub input_tokens: AtomicU64,
    /// Total output tokens consumed
    pub output_tokens: AtomicU64,
}

impl TokenUsage {
    /// Create a new token usage tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Add token usage from a single step
    pub fn add(&self, input: u64, output: u64) {
        self.input_tokens.fetch_add(input, Ordering::Relaxed);
        self.output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    /// Get total tokens (input + output)
    pub fn total(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed) + self.output_tokens.load(Ordering::Relaxed)
    }

    /// Get input tokens
    pub fn input(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed)
    }

    /// Get output tokens
    pub fn output(&self) -> u64 {
        self.output_tokens.load(Ordering::Relaxed)
    }

    /// Check if budget is exceeded
    pub fn is_budget_exceeded(&self, budget: Option<u64>) -> bool {
        if let Some(limit) = budget {
            self.total() >= limit
        } else {
            false
        }
    }

    /// Get remaining budget
    pub fn remaining(&self, budget: Option<u64>) -> Option<u64> {
        budget.map(|limit| limit.saturating_sub(self.total()))
    }
}

/// Response-driven agent execution result
#[derive(Debug, Clone)]
pub struct ReactiveResponse {
    /// Unique response ID
    pub id: Id,
    /// User's original request
    pub request: String,
    /// AI's text response
    pub content: String,
    /// Tool calls executed (if any)
    pub tool_calls: Vec<ToolCall>,
    /// Tool execution results
    pub tool_results: Vec<ToolResult>,
    /// Execution duration
    pub duration: std::time::Duration,
    /// Whether the task is completed
    pub completed: bool,
    /// Optional continuation prompt for multi-turn interactions
    pub continuation_prompt: Option<String>,
}

/// Tracks file operations for task completion verification
#[derive(Debug, Default, Clone)]
pub(crate) struct FileOperationTracker {
    /// Files created via Write tool
    pub created_files: Vec<String>,
    /// Files modified via Edit tool
    pub modified_files: Vec<String>,
}

impl FileOperationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track_tool_call(&mut self, tool_name: &str, result: &ToolResult) {
        if !result.success {
            return;
        }

        match tool_name {
            "Write" => {
                if let Some(path) = result.metadata.get("file_path").and_then(|v| v.as_str()) {
                    self.created_files.push(path.to_string());
                }
            }
            "Edit" => {
                if let Some(path) = result.metadata.get("file_path").and_then(|v| v.as_str()) {
                    self.modified_files.push(path.to_string());
                }
            }
            _ => {}
        }
    }

    pub fn has_file_operations(&self) -> bool {
        !self.created_files.is_empty() || !self.modified_files.is_empty()
    }

    #[allow(dead_code)] // Reserved for session reset functionality
    pub fn reset(&mut self) {
        self.created_files.clear();
        self.modified_files.clear();
    }
}
