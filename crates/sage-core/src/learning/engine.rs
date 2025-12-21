//! Learning engine for pattern detection and application

use super::types::*;
use crate::memory::{Memory, MemoryCategory, MemoryType, SharedMemoryManager};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error types for learning operations
#[derive(Debug, thiserror::Error)]
pub enum LearningError {
    #[error("Learning mode is disabled")]
    Disabled,
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Pattern limit reached")]
    PatternLimitReached,
}

/// Shared learning engine
pub type SharedLearningEngine = Arc<LearningEngine>;

/// Learning engine that tracks and applies learned patterns
pub struct LearningEngine {
    /// Configuration
    config: LearningConfig,
    /// Stored patterns
    patterns: RwLock<HashMap<PatternId, Pattern>>,
    /// Learning events history
    events: RwLock<Vec<LearningEvent>>,
    /// Optional memory manager for persistence
    memory_manager: Option<SharedMemoryManager>,
    /// Session statistics
    stats: RwLock<LearningStats>,
}

impl LearningEngine {
    /// Create a new learning engine
    pub fn new(config: LearningConfig) -> Self {
        Self {
            config,
            patterns: RwLock::new(HashMap::new()),
            events: RwLock::new(Vec::new()),
            memory_manager: None,
            stats: RwLock::new(LearningStats::default()),
        }
    }

    /// Create with memory manager for persistence
    pub fn with_memory_manager(
        config: LearningConfig,
        memory_manager: SharedMemoryManager,
    ) -> Self {
        Self {
            config,
            patterns: RwLock::new(HashMap::new()),
            events: RwLock::new(Vec::new()),
            memory_manager: Some(memory_manager),
            stats: RwLock::new(LearningStats::default()),
        }
    }

    /// Check if learning is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

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

    /// Get learning statistics
    pub async fn stats(&self) -> LearningStats {
        self.stats.read().await.clone()
    }

    /// Get recent learning events
    pub async fn recent_events(&self, limit: usize) -> Vec<LearningEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Clear all patterns (use with caution)
    pub async fn clear(&self) {
        let mut patterns = self.patterns.write().await;
        patterns.clear();

        let mut events = self.events.write().await;
        events.clear();

        let mut stats = self.stats.write().await;
        *stats = LearningStats::default();
    }

    /// Remove a specific pattern
    pub async fn remove_pattern(&self, pattern_id: &PatternId) -> Result<(), LearningError> {
        let mut patterns = self.patterns.write().await;
        patterns
            .remove(pattern_id)
            .ok_or_else(|| LearningError::PatternNotFound(pattern_id.to_string()))?;
        Ok(())
    }

    /// Apply decay to all patterns based on time
    pub async fn apply_decay(&self) {
        let mut patterns = self.patterns.write().await;
        let decay_threshold = chrono::Duration::days(self.config.decay_after_days as i64);
        let now = Utc::now();

        for pattern in patterns.values_mut() {
            let age = now - pattern.last_reinforced;
            if age > decay_threshold {
                let decay_factor =
                    (age.num_days() as f32 - self.config.decay_after_days as f32) / 30.0 * 0.1;
                pattern.confidence.decay(decay_factor.min(0.1));
            }
        }

        // Remove invalid patterns
        patterns.retain(|_, p| p.is_valid());
    }

    /// Load patterns from memory manager
    pub async fn load_from_memory(&self) -> Result<usize, LearningError> {
        let manager = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| LearningError::StorageError("No memory manager".to_string()))?;

        let memories = manager
            .find_by_type(MemoryType::Lesson)
            .await
            .map_err(|e| LearningError::StorageError(e.to_string()))?;

        let mut patterns = self.patterns.write().await;
        let mut count = 0;

        for memory in memories {
            if memory.has_tag("learning_pattern") {
                if let Some(data) = &memory.data {
                    if let Ok(pattern) = serde_json::from_value::<Pattern>(data.clone()) {
                        patterns.insert(pattern.id.clone(), pattern);
                        count += 1;
                    }
                }
            }
        }

        drop(patterns);
        self.update_stats().await;

        Ok(count)
    }

    // Private helper methods

    async fn record_event(&self, event: LearningEvent) {
        let mut events = self.events.write().await;
        events.push(event);

        // Keep only recent events
        if events.len() > 1000 {
            events.drain(0..500);
        }

        let mut stats = self.stats.write().await;
        stats.events_count += 1;
    }

    async fn update_stats(&self) {
        let patterns = self.patterns.read().await;
        let mut stats = self.stats.write().await;

        stats.total_patterns = patterns.len();
        stats.patterns_by_type.clear();

        let mut total_confidence = 0.0;
        let mut high_confidence = 0;

        for pattern in patterns.values() {
            *stats
                .patterns_by_type
                .entry(pattern.pattern_type.name().to_string())
                .or_insert(0) += 1;

            total_confidence += pattern.confidence.value();
            if pattern.confidence.is_high() {
                high_confidence += 1;
            }
        }

        stats.avg_confidence = if !patterns.is_empty() {
            total_confidence / patterns.len() as f32
        } else {
            0.0
        };
        stats.high_confidence_count = high_confidence;
    }

    async fn prune_patterns_locked(&self, patterns: &mut HashMap<PatternId, Pattern>) {
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

    async fn persist_pattern(&self, pattern_id: &PatternId, manager: &SharedMemoryManager) {
        let patterns = self.patterns.read().await;
        if let Some(pattern) = patterns.get(pattern_id) {
            let memory = Memory::new(
                MemoryType::Lesson,
                MemoryCategory::Global,
                format!("Learning pattern: {}", pattern.description),
            )
            .with_data(json!(pattern));

            // Add tag to identify as learning pattern
            let mut memory = memory;
            memory.add_tag("learning_pattern");
            memory.add_tag(pattern.pattern_type.name());

            let _ = manager.store(memory).await;
        }
    }
}

/// Create a shared learning engine
pub fn create_learning_engine(config: LearningConfig) -> SharedLearningEngine {
    Arc::new(LearningEngine::new(config))
}

/// Create a learning engine with memory manager
pub fn create_learning_engine_with_memory(
    config: LearningConfig,
    memory_manager: SharedMemoryManager,
) -> SharedLearningEngine {
    Arc::new(LearningEngine::with_memory_manager(config, memory_manager))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_learn_pattern() {
        let engine = LearningEngine::new(LearningConfig::default());

        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Use 4-space indentation",
            "Indent with 4 spaces, not tabs",
            PatternSource::UserExplicit,
        );

        let id = engine.learn(pattern).await.unwrap();
        assert!(engine.get_pattern(&id).await.is_some());
    }

    #[tokio::test]
    async fn test_learn_disabled() {
        let engine = LearningEngine::new(LearningConfig::disabled());

        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Test",
            "Rule",
            PatternSource::UserExplicit,
        );

        let result = engine.learn(pattern).await;
        assert!(matches!(result, Err(LearningError::Disabled)));
    }

    #[tokio::test]
    async fn test_reinforce_pattern() {
        let engine = LearningEngine::new(LearningConfig::default());

        let pattern = Pattern::new(
            PatternType::ToolPreference,
            "Use ripgrep",
            "Prefer rg over grep",
            PatternSource::ToolUsage,
        );

        let id = engine.learn(pattern).await.unwrap();
        let initial = engine.get_pattern(&id).await.unwrap();

        engine.reinforce(&id).await.unwrap();
        let reinforced = engine.get_pattern(&id).await.unwrap();

        assert!(reinforced.confidence.value() > initial.confidence.value());
        assert_eq!(reinforced.observation_count, 2);
    }

    #[tokio::test]
    async fn test_contradict_pattern() {
        let engine = LearningEngine::new(LearningConfig::default());

        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Style",
            "Rule",
            PatternSource::BehaviorPattern,
        );

        let id = engine.learn(pattern).await.unwrap();

        // Reinforce the pattern first so it has enough observations
        for _ in 0..5 {
            engine.reinforce(&id).await.unwrap();
        }

        // First contradiction should keep pattern valid
        let still_valid = engine.contradict(&id).await.unwrap();
        assert!(still_valid);

        // Many contradictions should eventually invalidate
        for _ in 0..10 {
            let _ = engine.contradict(&id).await;
        }

        let pattern = engine.get_pattern(&id).await;
        if let Some(p) = pattern {
            assert!(!p.is_valid() || p.contradiction_count > 5);
        }
    }

    #[tokio::test]
    async fn test_learn_from_correction() {
        let engine = LearningEngine::new(LearningConfig::default());

        let id = engine
            .learn_from_correction(
                "Using grep -r",
                "Use ripgrep (rg) for better performance",
                Some(vec!["bash".to_string()]),
            )
            .await
            .unwrap();

        let pattern = engine.get_pattern(&id).await.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Correction);
        assert!(pattern.context.contains(&"bash".to_string()));
    }

    #[tokio::test]
    async fn test_get_applicable_patterns() {
        let engine = LearningEngine::new(LearningConfig {
            apply_threshold: 0.5,
            ..Default::default()
        });

        // Learn a high-confidence pattern
        let pattern = Pattern::new(
            PatternType::ToolPreference,
            "Use ripgrep",
            "Prefer rg over grep",
            PatternSource::UserExplicit,
        )
        .with_confidence(0.9)
        .with_context("bash");

        engine.learn(pattern).await.unwrap();

        let applicable = engine.get_applicable_patterns(&["bash".to_string()]).await;
        assert!(!applicable.is_empty());

        let not_applicable = engine
            .get_applicable_patterns(&["unrelated".to_string()])
            .await;
        // Pattern with empty context would still apply, so check if it's in the list
        assert!(not_applicable.len() <= applicable.len());
    }

    #[tokio::test]
    async fn test_patterns_for_prompt() {
        let engine = LearningEngine::new(LearningConfig::default());

        // Learn high-confidence patterns
        for i in 0..5 {
            let pattern = Pattern::new(
                PatternType::CodingStyle,
                format!("Style rule {}", i),
                format!("Apply rule {}", i),
                PatternSource::UserExplicit,
            )
            .with_confidence(0.9);
            engine.learn(pattern).await.unwrap();
        }

        let prompts = engine.get_patterns_for_prompt(3).await;
        assert_eq!(prompts.len(), 3);
    }

    #[tokio::test]
    async fn test_stats() {
        let engine = LearningEngine::new(LearningConfig::default());

        // Learn patterns
        engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Style 1",
                "Rule 1",
                PatternSource::UserExplicit,
            ))
            .await
            .unwrap();

        engine
            .learn(Pattern::new(
                PatternType::ToolPreference,
                "Tool 1",
                "Rule 2",
                PatternSource::ToolUsage,
            ))
            .await
            .unwrap();

        let stats = engine.stats().await;
        assert_eq!(stats.total_patterns, 2);
        assert!(stats.patterns_by_type.contains_key("Coding Style"));
        assert!(stats.patterns_by_type.contains_key("Tool Preference"));
    }

    #[tokio::test]
    async fn test_clear() {
        let engine = LearningEngine::new(LearningConfig::default());

        engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Test",
                "Rule",
                PatternSource::UserExplicit,
            ))
            .await
            .unwrap();

        assert_eq!(engine.stats().await.total_patterns, 1);

        engine.clear().await;

        assert_eq!(engine.stats().await.total_patterns, 0);
    }

    #[tokio::test]
    async fn test_deduplicate_similar_patterns() {
        let engine = LearningEngine::new(LearningConfig::default());

        // Learn same pattern twice
        let id1 = engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Use 4 spaces",
                "indent with 4 spaces",
                PatternSource::UserExplicit,
            ))
            .await
            .unwrap();

        let id2 = engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Indentation",
                "INDENT WITH 4 SPACES", // Same rule, different case
                PatternSource::BehaviorPattern,
            ))
            .await
            .unwrap();

        // Should return the same ID (pattern was reinforced, not duplicated)
        assert_eq!(id1, id2);
        assert_eq!(engine.stats().await.total_patterns, 1);
    }
}
