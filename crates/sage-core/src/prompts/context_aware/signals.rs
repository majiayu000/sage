//! Phase detection signals

use std::collections::HashMap;

use super::ConversationPhase;

/// Signals used to detect conversation phase
#[derive(Debug, Clone, Default)]
pub struct PhaseSignals {
    /// Number of user messages
    pub user_message_count: usize,
    /// Number of assistant messages
    pub assistant_message_count: usize,
    /// Tools used in recent turns (tool_name -> count)
    pub recent_tool_usage: HashMap<String, usize>,
    /// Whether errors were encountered recently
    pub has_recent_errors: bool,
    /// Whether tests were run recently
    pub has_recent_tests: bool,
    /// Whether files were modified recently
    pub has_recent_modifications: bool,
    /// Whether in plan mode
    pub in_plan_mode: bool,
    /// Keywords detected in recent messages
    pub detected_keywords: Vec<String>,
    /// Explicit phase hint from user or system
    pub explicit_phase_hint: Option<ConversationPhase>,
}

impl PhaseSignals {
    /// Create new empty signals
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a tool usage
    pub fn record_tool_use(&mut self, tool_name: &str) {
        *self
            .recent_tool_usage
            .entry(tool_name.to_string())
            .or_insert(0) += 1;
    }

    /// Check if a specific tool was used
    pub fn tool_was_used(&self, tool_name: &str) -> bool {
        self.recent_tool_usage.contains_key(tool_name)
    }

    /// Get count of a specific tool usage
    pub fn tool_usage_count(&self, tool_name: &str) -> usize {
        self.recent_tool_usage.get(tool_name).copied().unwrap_or(0)
    }

    /// Add a detected keyword
    pub fn add_keyword(&mut self, keyword: impl Into<String>) {
        self.detected_keywords.push(keyword.into());
    }

    /// Check if a keyword was detected
    pub fn has_keyword(&self, keyword: &str) -> bool {
        self.detected_keywords
            .iter()
            .any(|k| k.eq_ignore_ascii_case(keyword))
    }
}
