//! Phase detection logic

use super::{ConversationPhase, PhaseSignals};

/// Detects the current conversation phase based on signals
#[derive(Debug, Clone)]
pub struct PhaseDetector {
    /// Tool names that indicate exploration
    exploration_tools: Vec<String>,
    /// Tool names that indicate implementation
    implementation_tools: Vec<String>,
    #[allow(dead_code)]
    /// Tool names that indicate testing
    testing_tools: Vec<String>,
    /// Keywords that suggest debugging
    debug_keywords: Vec<String>,
    /// Keywords that suggest completion
    completion_keywords: Vec<String>,
}

impl Default for PhaseDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseDetector {
    /// Create a new phase detector with default configuration
    pub fn new() -> Self {
        Self {
            exploration_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Task".to_string(),
            ],
            implementation_tools: vec![
                "Edit".to_string(),
                "Write".to_string(),
                "NotebookEdit".to_string(),
            ],
            testing_tools: vec!["Bash".to_string()],
            debug_keywords: vec![
                "error".to_string(),
                "bug".to_string(),
                "fix".to_string(),
                "issue".to_string(),
                "problem".to_string(),
                "fail".to_string(),
                "broken".to_string(),
            ],
            completion_keywords: vec![
                "done".to_string(),
                "complete".to_string(),
                "finished".to_string(),
                "summary".to_string(),
                "wrap up".to_string(),
            ],
        }
    }

    /// Detect the conversation phase from signals
    pub fn detect(&self, signals: &PhaseSignals) -> ConversationPhase {
        // Explicit hint takes precedence
        if let Some(phase) = signals.explicit_phase_hint {
            return phase;
        }

        // Plan mode means planning phase
        if signals.in_plan_mode {
            return ConversationPhase::Planning;
        }

        // Initial phase: very few messages
        if signals.user_message_count <= 1 && signals.assistant_message_count == 0 {
            return ConversationPhase::Initial;
        }

        // Check for debugging signals
        if signals.has_recent_errors || self.has_debug_keywords(signals) {
            return ConversationPhase::Debugging;
        }

        // Check for testing signals
        if signals.has_recent_tests {
            return ConversationPhase::Testing;
        }

        // Check for completion signals
        if self.has_completion_keywords(signals) {
            return ConversationPhase::Completing;
        }

        // Check tool usage patterns
        let exploration_count = self.count_tool_category(signals, &self.exploration_tools);
        let implementation_count = self.count_tool_category(signals, &self.implementation_tools);

        // If modifications were made, we're implementing
        if signals.has_recent_modifications || implementation_count > 0 {
            return ConversationPhase::Implementing;
        }

        // If mostly reading/searching, we're exploring
        if exploration_count > 0 {
            return ConversationPhase::Exploring;
        }

        // Default to initial for new conversations
        ConversationPhase::Initial
    }

    /// Count tool usage in a category
    fn count_tool_category(&self, signals: &PhaseSignals, tools: &[String]) -> usize {
        tools.iter().map(|t| signals.tool_usage_count(t)).sum()
    }

    /// Check if debug keywords are present
    fn has_debug_keywords(&self, signals: &PhaseSignals) -> bool {
        self.debug_keywords.iter().any(|kw| signals.has_keyword(kw))
    }

    /// Check if completion keywords are present
    fn has_completion_keywords(&self, signals: &PhaseSignals) -> bool {
        self.completion_keywords
            .iter()
            .any(|kw| signals.has_keyword(kw))
    }
}
