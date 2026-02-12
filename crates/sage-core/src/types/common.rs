//! Common types used throughout the Sage Agent system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for tasks, steps, and other entities
pub type Id = Uuid;

/// Token usage statistics for LLM calls
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    /// Number of input tokens
    pub input_tokens: u64,
    /// Number of output tokens
    pub output_tokens: u64,
    /// Tokens read from cache (Anthropic prompt caching)
    pub cache_read_tokens: Option<u64>,
    /// Tokens written to cache (Anthropic prompt caching)
    pub cache_write_tokens: Option<u64>,
    /// Cost estimate in USD (if available)
    pub cost_estimate: Option<f64>,
}

impl TokenUsage {
    /// Create a new TokenUsage instance
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens: None,
            cache_write_tokens: None,
            cost_estimate: None,
        }
    }

    /// Get total tokens (input + output)
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Add usage from another TokenUsage instance
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        match (self.cache_read_tokens, other.cache_read_tokens) {
            (Some(t1), Some(t2)) => self.cache_read_tokens = Some(t1 + t2),
            (None, Some(t)) => self.cache_read_tokens = Some(t),
            _ => {}
        }
        match (self.cache_write_tokens, other.cache_write_tokens) {
            (Some(t1), Some(t2)) => self.cache_write_tokens = Some(t1 + t2),
            (None, Some(t)) => self.cache_write_tokens = Some(t),
            _ => {}
        }
        if let Some(cost) = other.cost_estimate {
            *self.cost_estimate.get_or_insert(0.0) += cost;
        }
    }

    /// Check if this usage contains cache metrics
    pub fn has_cache_metrics(&self) -> bool {
        self.cache_read_tokens.is_some() || self.cache_write_tokens.is_some()
    }

    /// Get the effective input tokens (accounting for cache)
    /// Returns (regular_tokens, cached_tokens)
    pub fn get_cache_breakdown(&self) -> (u64, u64) {
        let cached = self.cache_read_tokens.unwrap_or(0);
        let regular = self.input_tokens.saturating_sub(cached);
        (regular, cached)
    }
}

// AgentState is now defined in agent::state module

/// Task execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Task ID
    pub id: Id,
    /// Task description
    pub description: String,
    /// Working directory
    pub working_dir: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub extra: HashMap<String, serde_json::Value>,
}

impl TaskMetadata {
    /// Create new task metadata
    pub fn new<S: Into<String>>(description: S, working_dir: S) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            working_dir: working_dir.into(),
            created_at: Utc::now(),
            completed_at: None,
            extra: HashMap::new(),
        }
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Get execution duration if completed
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed - self.created_at)
    }
}
