//! Learning mode types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// A learned pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Unique identifier
    pub id: PatternId,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Human-readable description
    pub description: String,
    /// The actual rule or behavior learned
    pub rule: String,
    /// Source of the pattern
    pub source: PatternSource,
    /// Confidence level
    pub confidence: Confidence,
    /// Number of times this pattern was observed
    pub observation_count: u32,
    /// Number of times this pattern was contradicted
    pub contradiction_count: u32,
    /// When the pattern was first learned
    pub created_at: DateTime<Utc>,
    /// When the pattern was last reinforced
    pub last_reinforced: DateTime<Utc>,
    /// Related context (tool names, file types, etc.)
    pub context: Vec<String>,
    /// Extra metadata
    pub metadata: HashMap<String, String>,
}

impl Pattern {
    /// Create a new pattern
    pub fn new(
        pattern_type: PatternType,
        description: impl Into<String>,
        rule: impl Into<String>,
        source: PatternSource,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: PatternId::new(),
            pattern_type,
            description: description.into(),
            rule: rule.into(),
            source,
            confidence: Confidence::default(),
            observation_count: 1,
            contradiction_count: 0,
            created_at: now,
            last_reinforced: now,
            context: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a correction pattern
    pub fn correction(what_was_wrong: &str, correct_behavior: &str) -> Self {
        Self::new(
            PatternType::Correction,
            format!("Avoid: {}", what_was_wrong),
            correct_behavior.to_string(),
            PatternSource::UserCorrection,
        )
    }

    /// Create a tool preference pattern
    pub fn tool_preference(tool_name: &str, preference: &str) -> Self {
        let mut pattern = Self::new(
            PatternType::ToolPreference,
            format!("Tool '{}' preference", tool_name),
            preference.to_string(),
            PatternSource::ToolUsage,
        );
        pattern.context.push(tool_name.to_string());
        pattern
    }

    /// Create a coding style pattern
    pub fn coding_style(style_aspect: &str, preference: &str) -> Self {
        Self::new(
            PatternType::CodingStyle,
            format!("Coding style: {}", style_aspect),
            preference.to_string(),
            PatternSource::CodeAnalysis,
        )
    }

    /// Reinforce the pattern (increase confidence)
    pub fn reinforce(&mut self) {
        self.observation_count += 1;
        self.confidence.reinforce(0.1);
        self.last_reinforced = Utc::now();
    }

    /// Record a contradiction
    pub fn contradict(&mut self) {
        self.contradiction_count += 1;
        self.confidence.decay(0.15);
    }

    /// Add context
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    /// Set initial confidence
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Confidence::new(confidence);
        self
    }

    /// Calculate effective relevance based on confidence and recency
    pub fn relevance_score(&self) -> f32 {
        let recency_days = (Utc::now() - self.last_reinforced).num_days() as f32;
        let recency_factor = 1.0 / (1.0 + recency_days / 30.0);

        let consistency_ratio = if self.observation_count > 0 {
            1.0 - (self.contradiction_count as f32 / self.observation_count as f32).min(0.5)
        } else {
            1.0
        };

        self.confidence.value() * recency_factor * consistency_ratio
    }

    /// Check if pattern is still valid (not too many contradictions)
    pub fn is_valid(&self) -> bool {
        self.contradiction_count < self.observation_count / 2 + 1 && self.confidence.value() > 0.2
    }
}

/// Learning event for tracking what was learned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Type of learning event
    pub event_type: LearningEventType,
    /// Pattern that was affected
    pub pattern_id: Option<PatternId>,
    /// Description of what happened
    pub description: String,
    /// Associated data
    pub data: HashMap<String, String>,
}

/// Type of learning event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningEventType {
    /// New pattern discovered
    PatternDiscovered,
    /// Existing pattern reinforced
    PatternReinforced,
    /// Pattern was contradicted
    PatternContradicted,
    /// Pattern invalidated
    PatternInvalidated,
    /// User explicitly taught something
    UserTeaching,
    /// Pattern applied successfully
    PatternApplied,
}

impl LearningEvent {
    /// Create a new learning event
    pub fn new(event_type: LearningEventType, description: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            pattern_id: None,
            description: description.into(),
            data: HashMap::new(),
        }
    }

    /// Associate with a pattern
    pub fn with_pattern(mut self, pattern_id: PatternId) -> Self {
        self.pattern_id = Some(pattern_id);
        self
    }

    /// Add data
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

/// Configuration for learning mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Whether learning mode is enabled
    pub enabled: bool,
    /// Minimum confidence threshold to apply a pattern
    pub apply_threshold: f32,
    /// Maximum number of patterns to store
    pub max_patterns: usize,
    /// Days before patterns start decaying
    pub decay_after_days: u32,
    /// Whether to learn from corrections automatically
    pub auto_learn_corrections: bool,
    /// Whether to learn from tool usage patterns
    pub auto_learn_tool_usage: bool,
    /// Whether to learn from code style
    pub auto_learn_code_style: bool,
    /// Store learning data persistently
    pub persistent: bool,
    /// Storage path for persistent learning
    pub storage_path: Option<std::path::PathBuf>,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            apply_threshold: 0.6,
            max_patterns: 500,
            decay_after_days: 30,
            auto_learn_corrections: true,
            auto_learn_tool_usage: true,
            auto_learn_code_style: true,
            persistent: true,
            storage_path: None,
        }
    }
}

impl LearningConfig {
    /// Create a disabled configuration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Create with persistent storage
    pub fn with_storage(storage_path: std::path::PathBuf) -> Self {
        Self {
            storage_path: Some(storage_path),
            ..Default::default()
        }
    }
}

/// Statistics about learning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningStats {
    /// Total patterns stored
    pub total_patterns: usize,
    /// Patterns by type
    pub patterns_by_type: HashMap<String, usize>,
    /// Average confidence across patterns
    pub avg_confidence: f32,
    /// Number of high-confidence patterns
    pub high_confidence_count: usize,
    /// Patterns applied in current session
    pub patterns_applied: usize,
    /// Learning events in current session
    pub events_count: usize,
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

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Use 4-space indentation",
            "Indent with 4 spaces",
            PatternSource::UserExplicit,
        );

        assert_eq!(pattern.pattern_type, PatternType::CodingStyle);
        assert_eq!(pattern.observation_count, 1);
        assert!(pattern.is_valid());
    }

    #[test]
    fn test_pattern_correction() {
        let pattern = Pattern::correction("Using tabs", "Use spaces for indentation");

        assert_eq!(pattern.pattern_type, PatternType::Correction);
        assert!(matches!(pattern.source, PatternSource::UserCorrection));
    }

    #[test]
    fn test_pattern_reinforcement() {
        let mut pattern = Pattern::new(
            PatternType::ToolPreference,
            "Prefer ripgrep",
            "Use rg instead of grep",
            PatternSource::ToolUsage,
        );

        let initial_confidence = pattern.confidence.value();
        pattern.reinforce();
        assert!(pattern.confidence.value() > initial_confidence);
        assert_eq!(pattern.observation_count, 2);
    }

    #[test]
    fn test_pattern_contradiction() {
        let mut pattern = Pattern::new(
            PatternType::CodingStyle,
            "Style preference",
            "Some rule",
            PatternSource::BehaviorPattern,
        );

        pattern.reinforce();
        pattern.reinforce();
        pattern.contradict();

        assert_eq!(pattern.contradiction_count, 1);
        assert!(pattern.is_valid()); // Still valid with few contradictions
    }

    #[test]
    fn test_pattern_invalidation() {
        let mut pattern = Pattern::new(
            PatternType::Custom,
            "Test pattern",
            "Test rule",
            PatternSource::BehaviorPattern,
        );

        // Many contradictions should invalidate
        for _ in 0..5 {
            pattern.contradict();
        }

        assert!(!pattern.is_valid());
    }

    #[test]
    fn test_learning_event() {
        let event = LearningEvent::new(
            LearningEventType::PatternDiscovered,
            "Learned new coding style preference",
        )
        .with_pattern(PatternId::from_string("pattern-1"))
        .with_data("file_type", "rust");

        assert_eq!(event.event_type, LearningEventType::PatternDiscovered);
        assert!(event.pattern_id.is_some());
        assert_eq!(event.data.get("file_type"), Some(&"rust".to_string()));
    }

    #[test]
    fn test_learning_config() {
        let config = LearningConfig::default();
        assert!(config.enabled);
        assert!(config.auto_learn_corrections);

        let disabled = LearningConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_pattern_relevance_score() {
        let pattern = Pattern::new(
            PatternType::Correction,
            "Test",
            "Rule",
            PatternSource::UserCorrection,
        )
        .with_confidence(0.9);

        let score = pattern.relevance_score();
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }
}
