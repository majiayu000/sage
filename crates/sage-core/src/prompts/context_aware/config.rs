//! Context-aware prompt configuration

use std::collections::HashMap;

use super::{ConversationPhase, PhasePrompts};

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
