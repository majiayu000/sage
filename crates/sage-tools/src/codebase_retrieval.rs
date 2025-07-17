//! Enhanced codebase retrieval tool for finding relevant code snippets

use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};

/// Tool for retrieving relevant code snippets from the codebase
pub struct CodebaseRetrievalTool {
    name: String,
    max_results: usize,
    max_file_size: usize,
    supported_extensions: HashSet<String>,
}

impl CodebaseRetrievalTool {
    pub fn new() -> Self {
        let mut supported_extensions = HashSet::new();
        // Programming languages
        supported_extensions.insert("rs".to_string());
        supported_extensions.insert("py".to_string());
        supported_extensions.insert("js".to_string());
        supported_extensions.insert("ts".to_string());
        supported_extensions.insert("java".to_string());
        supported_extensions.insert("cpp".to_string());
        supported_extensions.insert("c".to_string());
        supported_extensions.insert("h".to_string());
        supported_extensions.insert("go".to_string());
        supported_extensions.insert("rb".to_string());
        supported_extensions.insert("php".to_string());
        supported_extensions.insert("cs".to_string());
        supported_extensions.insert("swift".to_string());
        supported_extensions.insert("kt".to_string());
        supported_extensions.insert("scala".to_string());
        supported_extensions.insert("dart".to_string());
        
        // Config and markup
        supported_extensions.insert("json".to_string());
        supported_extensions.insert("toml".to_string());
        supported_extensions.insert("yaml".to_string());
        supported_extensions.insert("yml".to_string());
        supported_extensions.insert("xml".to_string());
        supported_extensions.insert("md".to_string());
        supported_extensions.insert("txt".to_string());
        
        Self {
            name: "codebase-retrieval".to_string(),
            max_results: 20,
            max_file_size: 1_000_000, // 1MB
            supported_extensions,
        }
    }

    /// Search for code snippets based on information request
    async fn search_codebase(&self, information_request: &str) -> Result<String, ToolError> {
        let current_dir = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current directory: {}", e)))?;
        
        // Extract search terms and analyze query
        let search_analysis = self.analyze_search_query(information_request);
        
        // Find all relevant files
        let files = self.find_relevant_files(&current_dir, &search_analysis)?;
        
        if files.is_empty() {
            return Ok(self.format_no_results_message(information_request));
        }

        // Search through files and rank results
        let mut results = Vec::new();
        for file_path in files.iter().take(50) { // Limit files to search
            if let Ok(matches) = self.search_file(&file_path, &search_analysis).await {
                if !matches.is_empty() {
                    results.extend(matches);
                }
            }
        }

        // Sort and limit results by relevance
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.max_results);

        if results.is_empty() {
            Ok(self.format_no_results_message(information_request))
        } else {
            Ok(self.format_search_results(&results, information_request))
        }
    }
    
    /// Analyze the search query to extract meaningful terms and patterns
    fn analyze_search_query(&self, query: &str) -> SearchAnalysis {
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

    fn clean_word(&self, word: &str) -> String {
        word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .to_lowercase()
    }

    fn is_camel_case(&self, word: &str) -> bool {
        word.chars().any(|c| c.is_uppercase()) && word.chars().any(|c| c.is_lowercase())
    }
    
    /// Find all relevant files in the directory
    fn find_relevant_files(&self, dir: &Path, search_analysis: &SearchAnalysis) -> Result<Vec<PathBuf>, ToolError> {
        let mut files = Vec::new();
        self.collect_files_recursive(dir, &mut files, 0, 5)?; // Max depth 5
        
        // Filter by file patterns if specified
        if !search_analysis.file_patterns.is_empty() {
            files.retain(|path| {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                search_analysis.file_patterns.iter().any(|pattern| {
                    file_name.contains(pattern)
                })
            });
        }

        Ok(files)
    }

    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>, depth: usize, max_depth: usize) -> Result<(), ToolError> {
        if depth > max_depth {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| ToolError::Io(e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                    if self.supported_extensions.contains(&extension.to_lowercase()) {
                        // Check file size
                        if let Ok(metadata) = fs::metadata(&path) {
                            if metadata.len() <= self.max_file_size as u64 {
                                files.push(path);
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Skip common directories that usually don't contain source code
                if !self.should_skip_directory(dir_name) {
                    self.collect_files_recursive(&path, files, depth + 1, max_depth)?;
                }
            }
        }
        
        Ok(())
    }

    fn should_skip_directory(&self, dir_name: &str) -> bool {
        matches!(dir_name, 
            ".git" | ".svn" | ".hg" |
            "node_modules" | "target" | "build" | "dist" | 
            "__pycache__" | ".pytest_cache" | 
            ".idea" | ".vscode" | 
            "coverage" | "htmlcov" |
            "tmp" | "temp" | "cache"
        ) || dir_name.starts_with('.')
    }
    
    /// Search within a single file for relevant content
    async fn search_file(&self, file_path: &Path, search_analysis: &SearchAnalysis) -> Result<Vec<SearchResult>, ToolError> {
        let content = fs::read_to_string(file_path).map_err(|e| ToolError::Io(e))?;
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
            if line.trim_start().starts_with("//") || line.trim_start().starts_with("#") || line.trim_start().starts_with("*") {
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

    fn format_no_results_message(&self, query: &str) -> String {
        format!(
            "No relevant code snippets found for: \"{}\"\n\n\
            💡 Try refining your search:\n\
            • Use specific function or class names\n\
            • Include file extensions (e.g., \"config.rs\")\n\
            • Try different keywords or synonyms\n\
            • Check if the files exist in the current directory\n\n\
            Supported file types: {}",
            query,
            self.supported_extensions.iter()
                .take(10)
                .map(|ext| format!(".{}", ext))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn format_search_results(&self, results: &[SearchResult], query: &str) -> String {
        let mut output = format!(
            "🔍 Found {} relevant code snippet(s) for: \"{}\"\n\n",
            results.len(),
            query
        );

        let mut file_groups: HashMap<PathBuf, Vec<&SearchResult>> = HashMap::new();
        for result in results {
            file_groups.entry(result.file_path.clone()).or_insert_with(Vec::new).push(result);
        }

        for (file_path, file_results) in file_groups {
            let relative_path = file_path.strip_prefix(std::env::current_dir().unwrap_or_default())
                .unwrap_or(&file_path)
                .to_string_lossy();
            
            output.push_str(&format!("📁 **{}**\n", relative_path));
            
            for result in file_results.iter().take(3) { // Max 3 results per file
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

impl Default for CodebaseRetrievalTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CodebaseRetrievalTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Enhanced codebase search engine that finds relevant code snippets using intelligent pattern matching. Supports function names, class names, keywords, and file patterns across multiple programming languages."
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let information_request = tool_call.arguments
            .get("information_request")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing required parameter: information_request".to_string()))?;

        match self.search_codebase(information_request).await {
            Ok(result) => Ok(ToolResult::success(&tool_call.id, self.name(), result)),
            Err(e) => Ok(ToolResult::error(&tool_call.id, self.name(), format!("Codebase retrieval failed: {}", e))),
        }
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "information_request": {
                        "type": "string",
                        "description": "Description of what you're looking for in the codebase. Can include function names, class names, keywords, or file patterns."
                    }
                },
                "required": ["information_request"]
            }),
        }
    }
}

/// Analysis of the search query
#[derive(Debug)]
struct SearchAnalysis {
    original_query: String,
    keywords: Vec<String>,
    function_patterns: Vec<String>,
    type_patterns: Vec<String>,
    file_patterns: Vec<String>,
}

/// A search result with relevance scoring
#[derive(Debug)]
struct SearchResult {
    file_path: PathBuf,
    line_number: usize,
    score: f64,
    matched_terms: Vec<String>,
    context: Vec<String>,
}