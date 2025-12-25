//! Result formatting utilities

use std::collections::HashMap;
use std::path::PathBuf;

use super::types::SearchResult;
use super::CodebaseRetrievalTool;

impl CodebaseRetrievalTool {
    pub(super) fn format_no_results_message(&self, query: &str) -> String {
        format!(
            "No relevant code snippets found for: \"{}\"\n\n\
            💡 Try refining your search:\n\
            • Use specific function or class names\n\
            • Include file extensions (e.g., \"config.rs\")\n\
            • Try different keywords or synonyms\n\
            • Check if the files exist in the current directory\n\n\
            Supported file types: {}",
            query,
            self.supported_extensions
                .iter()
                .take(10)
                .map(|ext| format!(".{}", ext))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub(super) fn format_search_results(&self, results: &[SearchResult], query: &str) -> String {
        let mut output = format!(
            "🔍 Found {} relevant code snippet(s) for: \"{}\"\n\n",
            results.len(),
            query
        );

        let mut file_groups: HashMap<PathBuf, Vec<&SearchResult>> = HashMap::new();
        for result in results {
            file_groups
                .entry(result.file_path.clone())
                .or_default()
                .push(result);
        }

        for (file_path, file_results) in file_groups {
            let relative_path = file_path
                .strip_prefix(std::env::current_dir().unwrap_or_default())
                .unwrap_or(&file_path)
                .to_string_lossy();

            output.push_str(&format!("📁 **{}**\n", relative_path));

            for result in file_results.iter().take(3) {
                // Max 3 results per file
                output.push_str(&format!(
                    "   Line {}: [Score: {:.1}] Matches: {}\n",
                    result.line_number,
                    result.score,
                    result.matched_terms.join(", ")
                ));

                for context_line in &result.context {
                    output.push_str(&format!("   {}\n", context_line));
                }
                output.push('\n');
            }
        }

        output
    }
}
