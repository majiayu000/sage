//! Pattern persistence and lifecycle management

use super::core::LearningEngine;
use super::error::LearningError;
use crate::learning::types::*;
use crate::memory::{Memory, MemoryCategory, MemoryType, SharedMemoryManager};
use chrono::Utc;
use serde_json::json;

impl LearningEngine {
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

    /// Persist a pattern to memory manager
    pub(super) async fn persist_pattern(
        &self,
        pattern_id: &PatternId,
        manager: &SharedMemoryManager,
    ) {
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
