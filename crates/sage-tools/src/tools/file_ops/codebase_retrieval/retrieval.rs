//! Core search orchestration

use sage_core::tools::base::ToolError;

use super::CodebaseRetrievalTool;

impl CodebaseRetrievalTool {
    /// Search for code snippets based on information request
    pub(super) async fn search_codebase(
        &self,
        information_request: &str,
    ) -> Result<String, ToolError> {
        let current_dir = std::env::current_dir().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get current directory: {}", e))
        })?;

        // Extract search terms and analyze query
        let search_analysis = self.analyze_search_query(information_request);

        // Find all relevant files
        let files = self.find_relevant_files(&current_dir, &search_analysis)?;

        if files.is_empty() {
            return Ok(self.format_no_results_message(information_request));
        }

        // Search through files and rank results
        let mut results = Vec::new();
        for file_path in files.iter().take(50) {
            // Limit files to search
            if let Ok(matches) = self.search_file(file_path, &search_analysis).await {
                if !matches.is_empty() {
                    results.extend(matches);
                }
            }
        }

        // Sort and limit results by relevance
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(self.max_results);

        if results.is_empty() {
            Ok(self.format_no_results_message(information_request))
        } else {
            Ok(self.format_search_results(&results, information_request))
        }
    }
}
