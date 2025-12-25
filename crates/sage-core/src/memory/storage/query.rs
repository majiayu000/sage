//! Query matching and scoring functions

use super::super::types::{Memory, MemoryQuery};

/// Check if a memory matches a query
pub fn matches_query(memory: &Memory, query: &MemoryQuery) -> bool {
    // Text filter - require at least partial match
    if let Some(ref text) = query.text {
        let text_lower = text.to_lowercase();
        let content_lower = memory.content.to_lowercase();

        // Check for substring match
        if !content_lower.contains(&text_lower) {
            // Check for word overlap
            let query_words: Vec<&str> = text_lower.split_whitespace().collect();
            let content_words: Vec<&str> = content_lower.split_whitespace().collect();

            let has_match = query_words
                .iter()
                .any(|qw| content_words.iter().any(|cw| cw.contains(qw)));

            if !has_match {
                return false;
            }
        }
    }

    // Type filter
    if let Some(ref mt) = query.memory_type {
        if &memory.memory_type != mt {
            return false;
        }
    }

    // Category filter
    if let Some(ref cat) = query.category {
        if &memory.category != cat {
            return false;
        }
    }

    // Tag filter
    if !query.tags.is_empty() {
        if !query.tags.iter().any(|t| memory.has_tag(t)) {
            return false;
        }
    }

    // Pinned filter
    if !query.include_pinned && memory.metadata.pinned {
        return false;
    }

    // Time filters
    if let Some(after) = query.created_after {
        if memory.metadata.created_at < after {
            return false;
        }
    }

    if let Some(after) = query.accessed_after {
        if memory.metadata.accessed_at < after {
            return false;
        }
    }

    true
}

/// Calculate content match score
pub fn calculate_content_score(memory: &Memory, query: &MemoryQuery) -> f32 {
    if let Some(ref text) = query.text {
        let text_lower = text.to_lowercase();
        let content_lower = memory.content.to_lowercase();

        // Check for exact match
        if content_lower.contains(&text_lower) {
            return 1.0;
        }

        // Check for word matches
        let query_words: Vec<&str> = text_lower.split_whitespace().collect();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        let mut matches = 0;
        for qw in &query_words {
            if content_words.iter().any(|cw| cw.contains(qw)) {
                matches += 1;
            }
        }

        if query_words.is_empty() {
            1.0
        } else {
            matches as f32 / query_words.len() as f32
        }
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_query_all() {
        let memory = Memory::fact("Test");
        let query = MemoryQuery::new();
        assert!(matches_query(&memory, &query));
    }

    #[test]
    fn test_calculate_content_score_exact() {
        let memory = Memory::fact("Rust programming language");
        let query = MemoryQuery::new().text("Rust");
        assert_eq!(calculate_content_score(&memory, &query), 1.0);
    }

    #[test]
    fn test_calculate_content_score_partial() {
        let memory = Memory::fact("Python is great");
        let query = MemoryQuery::new().text("Rust is great");
        let score = calculate_content_score(&memory, &query);
        assert!(score > 0.0 && score < 1.0);
    }
}
