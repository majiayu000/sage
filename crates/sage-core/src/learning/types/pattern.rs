//! Pattern type for learned behaviors

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::base::{Confidence, PatternId, PatternSource, PatternType};

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
    pub fn new(pattern_type: PatternType, description: impl Into<String>,
        rule: impl Into<String>, source: PatternSource) -> Self {
        let now = Utc::now();
        Self {
            id: PatternId::new(), pattern_type, description: description.into(),
            rule: rule.into(), source, confidence: Confidence::default(),
            observation_count: 1, contradiction_count: 0, created_at: now,
            last_reinforced: now, context: Vec::new(), metadata: HashMap::new(),
        }
    }

    /// Create a correction pattern
    pub fn correction(what_was_wrong: &str, correct_behavior: &str) -> Self {
        Self::new(PatternType::Correction, format!("Avoid: {}", what_was_wrong),
            correct_behavior.to_string(), PatternSource::UserCorrection)
    }

    /// Create a tool preference pattern
    pub fn tool_preference(tool_name: &str, preference: &str) -> Self {
        let mut pattern = Self::new(PatternType::ToolPreference,
            format!("Tool '{}' preference", tool_name),
            preference.to_string(), PatternSource::ToolUsage);
        pattern.context.push(tool_name.to_string());
        pattern
    }

    /// Create a coding style pattern
    pub fn coding_style(style_aspect: &str, preference: &str) -> Self {
        Self::new(PatternType::CodingStyle, format!("Coding style: {}", style_aspect),
            preference.to_string(), PatternSource::CodeAnalysis)
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(pattern.is_valid());
    }

    #[test]
    fn test_pattern_invalidation() {
        let mut pattern = Pattern::new(
            PatternType::Custom,
            "Test pattern",
            "Test rule",
            PatternSource::BehaviorPattern,
        );
        for _ in 0..5 {
            pattern.contradict();
        }
        assert!(!pattern.is_valid());
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
