//! Learning operations - pattern learning, reinforcement, and contradiction

use super::core::LearningEngine;
use super::error::LearningError;
use crate::learning::types::*;
use std::collections::HashMap;

impl LearningEngine {
    /// Learn a new pattern
    pub async fn learn(&self, pattern: Pattern) -> Result<PatternId, LearningError> {
        if !self.config.enabled {
            return Err(LearningError::Disabled);
        }

        let mut patterns = self.patterns.write().await;

        // Check pattern limit
        if patterns.len() >= self.config.max_patterns {
            // Remove lowest relevance pattern
            self.prune_patterns_locked(&mut patterns).await;
        }

        let pattern_id = pattern.id.clone();
        let pattern_type = pattern.pattern_type;

        // Check for similar existing pattern
        let existing = patterns.values().find(|p| {
            p.pattern_type == pattern.pattern_type
                && p.rule.to_lowercase() == pattern.rule.to_lowercase()
        });

        if let Some(existing) = existing {
            let id = existing.id.clone();
            drop(patterns);
            self.reinforce(&id).await?;
            return Ok(id);
        }

        patterns.insert(pattern_id.clone(), pattern);
        drop(patterns);

        // Record learning event
        self.record_event(
            LearningEvent::new(
                LearningEventType::PatternDiscovered,
                format!("Learned new {} pattern", pattern_type.name()),
            )
            .with_pattern(pattern_id.clone()),
        )
        .await;

        // Persist to memory if available
        if let Some(ref manager) = self.memory_manager {
            self.persist_pattern(&pattern_id, manager).await;
        }

        // Update stats
        self.update_stats().await;

        Ok(pattern_id)
    }

    /// Reinforce an existing pattern
    pub async fn reinforce(&self, pattern_id: &PatternId) -> Result<(), LearningError> {
        if !self.config.enabled {
            return Err(LearningError::Disabled);
        }

        let mut patterns = self.patterns.write().await;
        let pattern = patterns
            .get_mut(pattern_id)
            .ok_or_else(|| LearningError::PatternNotFound(pattern_id.to_string()))?;

        pattern.reinforce();

        self.record_event(
            LearningEvent::new(
                LearningEventType::PatternReinforced,
                format!("Pattern reinforced: {}", pattern.description),
            )
            .with_pattern(pattern_id.clone()),
        )
        .await;

        Ok(())
    }

    /// Record a contradiction to a pattern
    pub async fn contradict(&self, pattern_id: &PatternId) -> Result<bool, LearningError> {
        if !self.config.enabled {
            return Err(LearningError::Disabled);
        }

        let mut patterns = self.patterns.write().await;
        let pattern = patterns
            .get_mut(pattern_id)
            .ok_or_else(|| LearningError::PatternNotFound(pattern_id.to_string()))?;

        pattern.contradict();

        let still_valid = pattern.is_valid();

        if !still_valid {
            self.record_event(
                LearningEvent::new(
                    LearningEventType::PatternInvalidated,
                    format!("Pattern invalidated: {}", pattern.description),
                )
                .with_pattern(pattern_id.clone()),
            )
            .await;
        } else {
            self.record_event(
                LearningEvent::new(
                    LearningEventType::PatternContradicted,
                    format!("Pattern contradicted: {}", pattern.description),
                )
                .with_pattern(pattern_id.clone()),
            )
            .await;
        }

        Ok(still_valid)
    }

    /// Learn from a user correction
    pub async fn learn_from_correction(
        &self,
        what_was_wrong: &str,
        correct_behavior: &str,
        context: Option<Vec<String>>,
    ) -> Result<PatternId, LearningError> {
        if !self.config.auto_learn_corrections {
            return Err(LearningError::Disabled);
        }

        let mut pattern =
            Pattern::correction(what_was_wrong, correct_behavior).with_confidence(0.7); // Corrections start with higher confidence

        if let Some(ctx) = context {
            for c in ctx {
                pattern = pattern.with_context(c);
            }
        }

        self.learn(pattern).await
    }

    /// Learn from tool usage pattern
    pub async fn learn_from_tool_usage(
        &self,
        tool_name: &str,
        preference: &str,
    ) -> Result<PatternId, LearningError> {
        if !self.config.auto_learn_tool_usage {
            return Err(LearningError::Disabled);
        }

        let pattern = Pattern::tool_preference(tool_name, preference);
        self.learn(pattern).await
    }

    /// Learn coding style preference
    pub async fn learn_coding_style(
        &self,
        style_aspect: &str,
        preference: &str,
        file_type: Option<&str>,
    ) -> Result<PatternId, LearningError> {
        if !self.config.auto_learn_code_style {
            return Err(LearningError::Disabled);
        }

        let mut pattern = Pattern::coding_style(style_aspect, preference);
        if let Some(ft) = file_type {
            pattern = pattern.with_context(format!("file_type:{}", ft));
        }

        self.learn(pattern).await
    }

    /// Mark a pattern as successfully applied
    pub async fn mark_applied(&self, pattern_id: &PatternId) -> Result<(), LearningError> {
        self.reinforce(pattern_id).await?;

        self.record_event(
            LearningEvent::new(
                LearningEventType::PatternApplied,
                "Pattern successfully applied",
            )
            .with_pattern(pattern_id.clone()),
        )
        .await;

        let mut stats = self.stats.write().await;
        stats.patterns_applied += 1;

        Ok(())
    }

    /// Prune patterns when limit is reached
    pub(super) async fn prune_patterns_locked(&self, patterns: &mut HashMap<PatternId, Pattern>) {
        // Find pattern with lowest relevance
        if let Some((id, _)) = patterns
            .iter()
            .filter(|(_, p)| !p.confidence.is_high()) // Don't prune high-confidence patterns
            .min_by(|(_, a), (_, b)| {
                a.relevance_score()
                    .partial_cmp(&b.relevance_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            let id = id.clone();
            patterns.remove(&id);
        }
    }
}
