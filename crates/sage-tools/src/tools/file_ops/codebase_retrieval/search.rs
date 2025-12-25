//! File search and scoring logic

use sage_core::tools::base::ToolError;
use std::fs;
use std::path::Path;

use super::CodebaseRetrievalTool;
use super::types::{SearchAnalysis, SearchResult};

impl CodebaseRetrievalTool {
    /// Search within a single file for relevant content
    pub(super) async fn search_file(
        &self,
        file_path: &Path,
        search_analysis: &SearchAnalysis,
    ) -> Result<Vec<SearchResult>, ToolError> {
        let content = fs::read_to_string(file_path).map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read file for search '{}': {}",
                file_path.display(),
                e
            ))
        })?;
        let lines: Vec<&str> = content.lines().collect();
        let mut results = Vec::new();

        for (line_number, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let mut score = 0.0;
            let mut matched_terms = Vec::new();

            // Score function patterns (highest priority)
            for pattern in &search_analysis.function_patterns {
                if line_lower.contains(&pattern.to_lowercase()) {
                    score += 3.0;
                    matched_terms.push(pattern.clone());
                }
            }

            // Score type patterns (high priority)
            for pattern in &search_analysis.type_patterns {
                if line_lower.contains(&pattern.to_lowercase()) {
                    score += 2.0;
                    matched_terms.push(pattern.clone());
                }
            }

            // Score keywords (medium priority)
            for keyword in &search_analysis.keywords {
                if line_lower.contains(keyword) {
                    score += 1.0;
                    matched_terms.push(keyword.clone());
                }
            }

            // Bonus for exact matches
            if line_lower.contains(&search_analysis.original_query.to_lowercase()) {
                score += 2.0;
            }

            // Bonus for comments or documentation
            if line.trim_start().starts_with("//")
                || line.trim_start().starts_with("#")
                || line.trim_start().starts_with("*")
            {
                score += 0.5;
            }

            if score > 0.0 && !matched_terms.is_empty() {
                let context_start = line_number.saturating_sub(2);
                let context_end = std::cmp::min(line_number + 3, lines.len());

                let context_lines: Vec<String> = (context_start..context_end)
                    .map(|i| format!("{:4}: {}", i + 1, lines[i]))
                    .collect();

                results.push(SearchResult {
                    file_path: file_path.to_path_buf(),
                    line_number: line_number + 1,
                    score,
                    matched_terms,
                    context: context_lines,
                });
            }
        }

        Ok(results)
    }
}
