//! Memory relevance scoring types

use super::entries::Memory;
use serde::{Deserialize, Serialize};

/// Relevance score for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RelevanceScore {
    /// Content match score
    pub content_score: f32,
    /// Time decay score
    pub decay_score: f32,
    /// Combined score
    pub total: f32,
}

/// Memory with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryScore {
    /// The memory
    pub memory: Memory,
    /// Relevance score
    pub score: RelevanceScore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relevance_score_struct() {
        let score = RelevanceScore {
            content_score: 0.8,
            decay_score: 0.9,
            total: 0.72,
        };

        assert_eq!(score.total, 0.72);
    }

    #[test]
    fn test_memory_score() {
        let memory = Memory::fact("Test");
        let score = MemoryScore {
            memory: memory.clone(),
            score: RelevanceScore {
                content_score: 1.0,
                decay_score: 1.0,
                total: 1.0,
            },
        };

        assert_eq!(score.memory.content, "Test");
    }
}
