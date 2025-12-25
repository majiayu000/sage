//! Enhanced codebase retrieval tool for finding relevant code snippets
//!
//! **STATUS: DISABLED** - This is a Sage-specific tool not present in Claude Code.
//! Kept for potential future use but not registered in the default tool set.

mod formatting;
mod indexing;
mod query;
mod retrieval;
mod schema;
mod search;
mod types;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

/// Tool for retrieving relevant code snippets from the codebase
pub struct CodebaseRetrievalTool {
    name: String,
    max_results: usize,
    max_file_size: usize,
    supported_extensions: HashSet<String>,
}

impl CodebaseRetrievalTool {
    pub fn new() -> Self {
        Self {
            name: "codebase-retrieval".to_string(),
            max_results: 20,
            max_file_size: 1_000_000, // 1MB
            supported_extensions: indexing::get_supported_extensions(),
        }
    }
}

impl Default for CodebaseRetrievalTool {
    fn default() -> Self {
        Self::new()
    }
}
