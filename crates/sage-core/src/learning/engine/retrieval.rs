//! Pattern retrieval and query operations

use super::core::LearningEngine;
use crate::learning::types::*;

impl LearningEngine {
    /// Get all applicable patterns for a context
    pub async fn get_applicable_patterns(&self, context: &[String]) -> Vec<Pattern> {
        if !self.config.enabled {
            return Vec::new();
        }

        let patterns = self.patterns.read().await;
        let mut applicable: Vec<_> = patterns
            .values()
            .filter(|p| {
                p.is_valid()
                    && p.confidence.value() >= self.config.apply_threshold
                    && (p.context.is_empty() || p.context.iter().any(|c| context.contains(c)))
            })
            .cloned()
            .collect();

        // Sort by relevance
        applicable.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        applicable
    }

    /// Get patterns for system prompt inclusion
    pub async fn get_patterns_for_prompt(&self, limit: usize) -> Vec<String> {
        if !self.config.enabled {
            return Vec::new();
        }

        let patterns = self.patterns.read().await;
        let mut high_confidence: Vec<_> = patterns
            .values()
            .filter(|p| p.is_valid() && p.confidence.is_high())
            .collect();

        // Sort by relevance score
        high_confidence.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        high_confidence
            .iter()
            .take(limit)
            .map(|p| format!("[{}] {}: {}", p.pattern_type.name(), p.description, p.rule))
            .collect()
    }

    /// Get a specific pattern
    pub async fn get_pattern(&self, pattern_id: &PatternId) -> Option<Pattern> {
        let patterns = self.patterns.read().await;
        patterns.get(pattern_id).cloned()
    }

    /// Get all patterns of a specific type
    pub async fn get_patterns_by_type(&self, pattern_type: PatternType) -> Vec<Pattern> {
        let patterns = self.patterns.read().await;
        patterns
            .values()
            .filter(|p| p.pattern_type == pattern_type && p.is_valid())
            .cloned()
            .collect()
    }
}
