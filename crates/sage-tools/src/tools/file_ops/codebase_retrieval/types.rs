//! Data types for codebase retrieval

use std::path::PathBuf;

/// Analysis of the search query
#[derive(Debug)]
pub struct SearchAnalysis {
    pub original_query: String,
    pub keywords: Vec<String>,
    pub function_patterns: Vec<String>,
    pub type_patterns: Vec<String>,
    pub file_patterns: Vec<String>,
}

/// A search result with relevance scoring
#[derive(Debug)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub score: f64,
    pub matched_terms: Vec<String>,
    pub context: Vec<String>,
}
