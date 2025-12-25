//! Query analysis and pattern detection

use super::types::SearchAnalysis;
use super::CodebaseRetrievalTool;

impl CodebaseRetrievalTool {
    /// Analyze the search query to extract meaningful terms and patterns
    pub(super) fn analyze_search_query(&self, query: &str) -> SearchAnalysis {
        let words: Vec<String> = query
            .split_whitespace()
            .filter(|word| word.len() > 2)
            .map(|word| self.clean_word(word))
            .filter(|word| !word.is_empty())
            .collect();

        let mut keywords = Vec::new();
        let mut function_patterns = Vec::new();
        let mut type_patterns = Vec::new();
        let mut file_patterns = Vec::new();

        for word in &words {
            // Detect function patterns (ending with parentheses or containing underscore/camelCase)
            if word.contains('(') || word.contains('_') || self.is_camel_case(word) {
                function_patterns.push(word.clone());
            }
            // Detect type patterns (capitalized words)
            else if word.chars().next().unwrap_or('a').is_uppercase() {
                type_patterns.push(word.clone());
            }
            // Detect file patterns (containing dots or specific extensions)
            else if word.contains('.') {
                file_patterns.push(word.clone());
            }
            // General keywords
            else {
                keywords.push(word.clone());
            }
        }

        SearchAnalysis {
            original_query: query.to_string(),
            keywords,
            function_patterns,
            type_patterns,
            file_patterns,
        }
    }

    pub(super) fn clean_word(&self, word: &str) -> String {
        word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .to_lowercase()
    }

    pub(super) fn is_camel_case(&self, word: &str) -> bool {
        word.chars().any(|c| c.is_uppercase()) && word.chars().any(|c| c.is_lowercase())
    }
}
