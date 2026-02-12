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

mod config;
mod detector;
mod phase;
mod prompts;
mod signals;

pub use config::ContextAwareConfig;
pub use detector::PhaseDetector;
pub use phase::ConversationPhase;
pub use prompts::PhasePrompts;
pub use signals::PhaseSignals;

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
