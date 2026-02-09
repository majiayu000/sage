//! Context-aware prompt adjustment system
//!
//! This module provides dynamic prompt adjustment based on conversation phase,
//! following Claude Code's design pattern of context-sensitive guidance.
//!
//! # Conversation Phases
//!
//! The system recognizes different phases of a conversation and adjusts
//! prompts accordingly:
//!
//! - **Initial**: Fresh conversation, focus on understanding the request
//! - **Exploring**: Gathering context, reading files, searching codebase
//! - **Planning**: Designing implementation approach
//! - **Implementing**: Writing code, making changes
//! - **Debugging**: Fixing errors, investigating issues
//! - **Testing**: Running tests, verifying behavior
//! - **Reviewing**: Code review, final checks
//! - **Completing**: Wrapping up, summarizing work done
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::prompts::{ConversationPhase, PhaseDetector, PhasePrompts};
//!
//! // Detect phase from conversation history
//! let detector = PhaseDetector::new();
//! let phase = detector.detect(&messages, &tool_calls);
//!
//! // Get phase-specific prompt fragment
//! let fragment = PhasePrompts::for_phase(phase);
//! ```

use std::collections::HashMap;
use std::fmt;

/// Conversation phases that influence prompt behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConversationPhase {
    /// Fresh conversation, understanding the request
    Initial,
    /// Gathering context, reading files, searching codebase
    Exploring,
    /// Designing implementation approach
    Planning,
    /// Writing code, making changes
    Implementing,
    /// Fixing errors, investigating issues
    Debugging,
    /// Running tests, verifying behavior
    Testing,
    /// Code review, final checks
    Reviewing,
    /// Wrapping up, summarizing work done
    Completing,
}

impl fmt::Display for ConversationPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversationPhase::Initial => write!(f, "Initial"),
            ConversationPhase::Exploring => write!(f, "Exploring"),
            ConversationPhase::Planning => write!(f, "Planning"),
            ConversationPhase::Implementing => write!(f, "Implementing"),
            ConversationPhase::Debugging => write!(f, "Debugging"),
            ConversationPhase::Testing => write!(f, "Testing"),
            ConversationPhase::Reviewing => write!(f, "Reviewing"),
            ConversationPhase::Completing => write!(f, "Completing"),
        }
    }
}

impl ConversationPhase {
    /// Get all phases in typical workflow order
    pub fn workflow_order() -> &'static [ConversationPhase] {
        &[
            ConversationPhase::Initial,
            ConversationPhase::Exploring,
            ConversationPhase::Planning,
            ConversationPhase::Implementing,
            ConversationPhase::Testing,
            ConversationPhase::Debugging,
            ConversationPhase::Reviewing,
            ConversationPhase::Completing,
        ]
    }

    /// Check if this phase is read-only (no file modifications expected)
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            ConversationPhase::Initial
                | ConversationPhase::Exploring
                | ConversationPhase::Planning
                | ConversationPhase::Reviewing
        )
    }

    /// Check if this phase involves active coding
    pub fn is_coding_phase(&self) -> bool {
        matches!(
            self,
            ConversationPhase::Implementing | ConversationPhase::Debugging
        )
    }
}

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
            testing_tools: vec!["Bash".to_string()], // Bash is often used for running tests
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

/// Phase-specific prompt fragments
pub struct PhasePrompts;

impl PhasePrompts {
    /// Get the prompt fragment for a specific phase
    pub fn for_phase(phase: ConversationPhase) -> &'static str {
        match phase {
            ConversationPhase::Initial => Self::INITIAL,
            ConversationPhase::Exploring => Self::EXPLORING,
            ConversationPhase::Planning => Self::PLANNING,
            ConversationPhase::Implementing => Self::IMPLEMENTING,
            ConversationPhase::Debugging => Self::DEBUGGING,
            ConversationPhase::Testing => Self::TESTING,
            ConversationPhase::Reviewing => Self::REVIEWING,
            ConversationPhase::Completing => Self::COMPLETING,
        }
    }

    /// Initial phase: Understanding the request
    pub const INITIAL: &'static str = r#"## Current Phase: Understanding Request

Focus on understanding what the user needs:
- Read the request carefully before taking action
- If the task is unclear, ask ONE clarifying question
- For clear tasks, start working immediately
- Don't over-plan simple tasks"#;

    /// Exploring phase: Gathering context
    pub const EXPLORING: &'static str = r#"## Current Phase: Exploring Codebase

You are gathering context about the codebase:
- Use ${TASK_TOOL_NAME} with Explore agents for broad searches
- Read relevant files to understand the structure
- Look for existing patterns to follow
- Note dependencies and relationships
- Don't start implementing until you understand the context"#;

    /// Planning phase: Designing approach
    pub const PLANNING: &'static str = r#"## Current Phase: Planning Implementation

You are designing the implementation approach:
- Consider multiple approaches before choosing
- Keep the solution as simple as possible
- Identify files that need to be modified
- Think about edge cases and error handling
- Write your plan to the plan file"#;

    /// Implementing phase: Writing code
    pub const IMPLEMENTING: &'static str = r#"## Current Phase: Implementing Changes

You are actively writing code:
- Follow existing code patterns and style
- Make minimal, focused changes
- Don't over-engineer or add unnecessary features
- Test your changes as you go
- Keep commits atomic and well-described"#;

    /// Debugging phase: Fixing issues
    pub const DEBUGGING: &'static str = r#"## Current Phase: Debugging

You are investigating and fixing issues:
- Read error messages carefully
- Identify the root cause before fixing
- Make minimal changes to fix the issue
- Verify the fix doesn't break other things
- Add tests to prevent regression if appropriate"#;

    /// Testing phase: Verifying behavior
    pub const TESTING: &'static str = r#"## Current Phase: Testing

You are verifying the implementation:
- Run existing tests to check for regressions
- Test the specific functionality you changed
- Check edge cases and error conditions
- Fix any failing tests before moving on"#;

    /// Reviewing phase: Final checks
    pub const REVIEWING: &'static str = r#"## Current Phase: Reviewing

You are doing final checks:
- Review the changes for correctness
- Check for any missed requirements
- Ensure code quality and style consistency
- Verify all tests pass
- Prepare summary of changes made"#;

    /// Completing phase: Wrapping up
    pub const COMPLETING: &'static str = r#"## Current Phase: Completing

You are wrapping up the task:
- Summarize what was accomplished
- Note any remaining items or follow-ups
- Ensure all changes are committed if requested
- Provide clear next steps if applicable"#;

    /// Get a compact reminder for the phase (for injection into messages)
    pub fn compact_reminder(phase: ConversationPhase) -> String {
        match phase {
            ConversationPhase::Initial => {
                "Phase: Initial - Focus on understanding the request".to_string()
            }
            ConversationPhase::Exploring => {
                "Phase: Exploring - Gather context before implementing".to_string()
            }
            ConversationPhase::Planning => {
                "Phase: Planning - Design approach, write to plan file".to_string()
            }
            ConversationPhase::Implementing => {
                "Phase: Implementing - Write minimal, focused code".to_string()
            }
            ConversationPhase::Debugging => {
                "Phase: Debugging - Find root cause, make minimal fix".to_string()
            }
            ConversationPhase::Testing => {
                "Phase: Testing - Verify changes, check for regressions".to_string()
            }
            ConversationPhase::Reviewing => {
                "Phase: Reviewing - Final checks before completion".to_string()
            }
            ConversationPhase::Completing => {
                "Phase: Completing - Summarize and wrap up".to_string()
            }
        }
    }
}

/// Context-aware prompt configuration
#[derive(Debug, Clone)]
pub struct ContextAwareConfig {
    /// Whether to include phase-specific prompts
    pub include_phase_prompts: bool,
    /// Whether to include compact reminders in messages
    pub include_compact_reminders: bool,
    /// Minimum messages before phase detection activates
    pub min_messages_for_detection: usize,
    /// Custom phase overrides
    pub phase_overrides: HashMap<ConversationPhase, String>,
}

impl Default for ContextAwareConfig {
    fn default() -> Self {
        Self {
            include_phase_prompts: true,
            include_compact_reminders: false,
            min_messages_for_detection: 2,
            phase_overrides: HashMap::new(),
        }
    }
}

impl ContextAwareConfig {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable phase prompts
    pub fn with_phase_prompts(mut self, enabled: bool) -> Self {
        self.include_phase_prompts = enabled;
        self
    }

    /// Enable compact reminders
    pub fn with_compact_reminders(mut self, enabled: bool) -> Self {
        self.include_compact_reminders = enabled;
        self
    }

    /// Set minimum messages for detection
    pub fn with_min_messages(mut self, count: usize) -> Self {
        self.min_messages_for_detection = count;
        self
    }

    /// Add a custom phase override
    pub fn with_phase_override(
        mut self,
        phase: ConversationPhase,
        prompt: impl Into<String>,
    ) -> Self {
        self.phase_overrides.insert(phase, prompt.into());
        self
    }

    /// Get the prompt for a phase (with override support)
    pub fn get_phase_prompt(&self, phase: ConversationPhase) -> &str {
        self.phase_overrides
            .get(&phase)
            .map(|s| s.as_str())
            .unwrap_or_else(|| PhasePrompts::for_phase(phase))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_display() {
        assert_eq!(ConversationPhase::Initial.to_string(), "Initial");
        assert_eq!(ConversationPhase::Implementing.to_string(), "Implementing");
    }

    #[test]
    fn test_phase_is_read_only() {
        assert!(ConversationPhase::Initial.is_read_only());
        assert!(ConversationPhase::Exploring.is_read_only());
        assert!(ConversationPhase::Planning.is_read_only());
        assert!(!ConversationPhase::Implementing.is_read_only());
        assert!(!ConversationPhase::Debugging.is_read_only());
    }

    #[test]
    fn test_phase_is_coding() {
        assert!(!ConversationPhase::Initial.is_coding_phase());
        assert!(ConversationPhase::Implementing.is_coding_phase());
        assert!(ConversationPhase::Debugging.is_coding_phase());
    }

    #[test]
    fn test_phase_signals_tool_tracking() {
        let mut signals = PhaseSignals::new();
        signals.record_tool_use("Read");
        signals.record_tool_use("Read");
        signals.record_tool_use("Edit");

        assert!(signals.tool_was_used("Read"));
        assert!(signals.tool_was_used("Edit"));
        assert!(!signals.tool_was_used("Write"));
        assert_eq!(signals.tool_usage_count("Read"), 2);
        assert_eq!(signals.tool_usage_count("Edit"), 1);
    }

    #[test]
    fn test_phase_signals_keywords() {
        let mut signals = PhaseSignals::new();
        signals.add_keyword("error");
        signals.add_keyword("Fix");

        assert!(signals.has_keyword("error"));
        assert!(signals.has_keyword("ERROR")); // case insensitive
        assert!(signals.has_keyword("fix"));
        assert!(!signals.has_keyword("bug"));
    }

    #[test]
    fn test_detector_initial_phase() {
        let detector = PhaseDetector::new();
        let signals = PhaseSignals {
            user_message_count: 1,
            assistant_message_count: 0,
            ..Default::default()
        };

        assert_eq!(detector.detect(&signals), ConversationPhase::Initial);
    }

    #[test]
    fn test_detector_plan_mode() {
        let detector = PhaseDetector::new();
        let signals = PhaseSignals {
            user_message_count: 5,
            assistant_message_count: 4,
            in_plan_mode: true,
            ..Default::default()
        };

        assert_eq!(detector.detect(&signals), ConversationPhase::Planning);
    }

    #[test]
    fn test_detector_debugging_with_errors() {
        let detector = PhaseDetector::new();
        let signals = PhaseSignals {
            user_message_count: 5,
            assistant_message_count: 4,
            has_recent_errors: true,
            ..Default::default()
        };

        assert_eq!(detector.detect(&signals), ConversationPhase::Debugging);
    }

    #[test]
    fn test_detector_debugging_with_keywords() {
        let detector = PhaseDetector::new();
        let mut signals = PhaseSignals {
            user_message_count: 5,
            assistant_message_count: 4,
            ..Default::default()
        };
        signals.add_keyword("error");

        assert_eq!(detector.detect(&signals), ConversationPhase::Debugging);
    }

    #[test]
    fn test_detector_implementing() {
        let detector = PhaseDetector::new();
        let signals = PhaseSignals {
            user_message_count: 5,
            assistant_message_count: 4,
            has_recent_modifications: true,
            ..Default::default()
        };

        assert_eq!(detector.detect(&signals), ConversationPhase::Implementing);
    }

    #[test]
    fn test_detector_exploring() {
        let detector = PhaseDetector::new();
        let mut signals = PhaseSignals {
            user_message_count: 3,
            assistant_message_count: 2,
            ..Default::default()
        };
        signals.record_tool_use("Read");
        signals.record_tool_use("Glob");

        assert_eq!(detector.detect(&signals), ConversationPhase::Exploring);
    }

    #[test]
    fn test_detector_explicit_hint() {
        let detector = PhaseDetector::new();
        let signals = PhaseSignals {
            user_message_count: 10,
            assistant_message_count: 9,
            explicit_phase_hint: Some(ConversationPhase::Reviewing),
            ..Default::default()
        };

        assert_eq!(detector.detect(&signals), ConversationPhase::Reviewing);
    }

    #[test]
    fn test_phase_prompts_exist() {
        // Verify all phases have prompts
        for phase in ConversationPhase::workflow_order() {
            let prompt = PhasePrompts::for_phase(*phase);
            assert!(!prompt.is_empty());
            assert!(prompt.contains("Phase:"));
        }
    }

    #[test]
    fn test_compact_reminders() {
        let reminder = PhasePrompts::compact_reminder(ConversationPhase::Implementing);
        assert!(reminder.contains("Implementing"));
        assert!(reminder.contains("minimal"));
    }

    #[test]
    fn test_config_phase_override() {
        let config = ContextAwareConfig::new()
            .with_phase_override(ConversationPhase::Initial, "Custom initial prompt");

        assert_eq!(
            config.get_phase_prompt(ConversationPhase::Initial),
            "Custom initial prompt"
        );
        // Non-overridden phases use default
        assert_eq!(
            config.get_phase_prompt(ConversationPhase::Exploring),
            PhasePrompts::EXPLORING
        );
    }
}
