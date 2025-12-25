//! Core learning engine struct and basic operations

use super::error::LearningError;
use crate::learning::types::*;
use crate::memory::SharedMemoryManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared learning engine
pub type SharedLearningEngine = Arc<LearningEngine>;

/// Learning engine that tracks and applies learned patterns
pub struct LearningEngine {
    /// Configuration
    pub(super) config: LearningConfig,
    /// Stored patterns
    pub(super) patterns: RwLock<HashMap<PatternId, Pattern>>,
    /// Learning events history
    pub(super) events: RwLock<Vec<LearningEvent>>,
    /// Optional memory manager for persistence
    pub(super) memory_manager: Option<SharedMemoryManager>,
    /// Session statistics
    pub(super) stats: RwLock<LearningStats>,
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

    /// Record a learning event
    pub(super) async fn record_event(&self, event: LearningEvent) {
        let mut events = self.events.write().await;
        events.push(event);

        // Keep only recent events
        if events.len() > 1000 {
            events.drain(0..500);
        }

        let mut stats = self.stats.write().await;
        stats.events_count += 1;
    }

    /// Update statistics based on current patterns
    pub(super) async fn update_stats(&self) {
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
