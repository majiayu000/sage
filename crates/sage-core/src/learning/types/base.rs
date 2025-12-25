//! Base types for learning patterns

use serde::{Deserialize, Serialize};

/// Learning pattern identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatternId(pub String);

impl PatternId {
    /// Create a new random pattern ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the ID string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for PatternId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of pattern detected by learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternType {
    /// User correction of agent behavior
    Correction,
    /// Tool usage preference
    ToolPreference,
    /// Coding style preference (formatting, naming)
    CodingStyle,
    /// Error handling pattern
    ErrorHandling,
    /// Communication style preference
    CommunicationStyle,
    /// Workflow preference (commit frequency, test-first)
    WorkflowPreference,
    /// Project-specific pattern
    ProjectSpecific,
    /// Custom pattern type
    Custom,
}

impl PatternType {
    /// Get display name
    pub fn name(&self) -> &str {
        match self {
            Self::Correction => "Correction",
            Self::ToolPreference => "Tool Preference",
            Self::CodingStyle => "Coding Style",
            Self::ErrorHandling => "Error Handling",
            Self::CommunicationStyle => "Communication",
            Self::WorkflowPreference => "Workflow",
            Self::ProjectSpecific => "Project Specific",
            Self::Custom => "Custom",
        }
    }
}

/// Confidence level for a pattern
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Confidence(f32);

impl Confidence {
    /// Create a new confidence value (clamped to 0.0-1.0)
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the confidence value
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Low confidence threshold
    pub fn is_low(&self) -> bool {
        self.0 < 0.4
    }

    /// Medium confidence threshold
    pub fn is_medium(&self) -> bool {
        self.0 >= 0.4 && self.0 < 0.7
    }

    /// High confidence threshold
    pub fn is_high(&self) -> bool {
        self.0 >= 0.7
    }

    /// Increase confidence based on observations
    pub fn reinforce(&mut self, amount: f32) {
        self.0 = (self.0 + amount * (1.0 - self.0)).clamp(0.0, 1.0);
    }

    /// Decrease confidence (decay over time or contradiction)
    pub fn decay(&mut self, amount: f32) {
        self.0 = (self.0 - amount * self.0).clamp(0.0, 1.0);
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self(0.5)
    }
}

/// Source of a learning pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternSource {
    /// Explicitly stated by user
    UserExplicit,
    /// Inferred from user correction
    UserCorrection,
    /// Inferred from repeated behavior
    BehaviorPattern,
    /// Inferred from tool usage patterns
    ToolUsage,
    /// Inferred from code style
    CodeAnalysis,
    /// Imported from configuration
    Configuration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_id() {
        let id1 = PatternId::new();
        let id2 = PatternId::new();
        assert_ne!(id1, id2);

        let id3 = PatternId::from_string("test-id");
        assert_eq!(id3.as_str(), "test-id");
    }

    #[test]
    fn test_confidence() {
        let mut conf = Confidence::new(0.5);
        assert!(conf.is_medium());

        conf.reinforce(0.3);
        assert!(conf.value() > 0.5);

        conf.decay(0.2);
        assert!(conf.value() < 1.0);
    }

    #[test]
    fn test_confidence_clamping() {
        let conf_high = Confidence::new(1.5);
        assert_eq!(conf_high.value(), 1.0);

        let conf_low = Confidence::new(-0.5);
        assert_eq!(conf_low.value(), 0.0);
    }
}
