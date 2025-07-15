//! Codebase retrieval tool for finding relevant code snippets

use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};

/// Tool for retrieving relevant code snippets from the codebase
pub struct CodebaseRetrievalTool {
    name: String,
}

impl CodebaseRetrievalTool {
    pub fn new() -> Self {
        Self {
            name: "codebase-retrieval".to_string(),
        }
    }

    /// Search for code snippets based on information request
    async fn search_codebase(&self, information_request: &str) -> Result<String, ToolError> {
        // This is a simplified implementation
        // In a real implementation, this would use advanced code search techniques
        // like semantic search, AST parsing, etc.
        
        let current_dir = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current directory: {}", e)))?;
        
        // Search for relevant files based on keywords
        let keywords = self.extract_keywords(information_request);
        let mut results = Vec::new();
        
        // Search common code file extensions
        let extensions = vec!["rs", "py", "js", "ts", "java", "cpp", "c", "h", "go", "rb"];
        
        for extension in extensions {
            if let Ok(files) = self.find_files_with_extension(&current_dir, extension) {
                for file_path in files.iter().take(10) { // Limit to 10 files per extension
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        if self.content_matches_keywords(&content, &keywords) {
                            let relative_path = file_path.strip_prefix(&current_dir)
                                .unwrap_or(file_path)
                                .to_string_lossy();
                            
                            // Extract relevant snippets
                            let snippets = self.extract_relevant_snippets(&content, &keywords);
                            if !snippets.is_empty() {
                                results.push(format!(
                                    "Path: {}\n{}",
                                    relative_path,
                                    snippets.join("\n")
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        if results.is_empty() {
            Ok(format!(
                "No relevant code snippets found for: {}\n\nTry being more specific about:\n- Function names\n- Class names\n- File names\n- Specific functionality",
                information_request
            ))
        } else {
            Ok(format!(
                "The following code sections were retrieved:\n{}",
                results.join("\n\n")
            ))
        }
    }
    
    /// Extract keywords from the information request
    fn extract_keywords(&self, request: &str) -> Vec<String> {
        request
            .split_whitespace()
            .filter(|word| word.len() > 2)
            .map(|word| word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|word| !word.is_empty())
            .collect()
    }
    
    /// Find files with specific extension
    fn find_files_with_extension(&self, dir: &Path, extension: &str) -> Result<Vec<std::path::PathBuf>, ToolError> {
        let mut files = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            files.push(path);
                        }
                    }
                } else if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                    // Recursively search subdirectories (limit depth)
                    if let Ok(mut sub_files) = self.find_files_with_extension(&path, extension) {
                        files.append(&mut sub_files);
                    }
                }
                
                // Limit total files to prevent overwhelming results
                if files.len() > 50 {
                    break;
                }
            }
        }
        
        Ok(files)
    }
    
    /// Check if content matches keywords
    fn content_matches_keywords(&self, content: &str, keywords: &[String]) -> bool {
        let content_lower = content.to_lowercase();
        keywords.iter().any(|keyword| content_lower.contains(keyword))
    }
    
    /// Extract relevant code snippets
    fn extract_relevant_snippets(&self, content: &str, keywords: &[String]) -> Vec<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut snippets = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            if keywords.iter().any(|keyword| line_lower.contains(keyword)) {
                // Extract context around the matching line
                let start = i.saturating_sub(2);
                let end = std::cmp::min(i + 3, lines.len());
                
                let snippet_lines: Vec<String> = (start..end)
                    .map(|idx| format!("{:5}	{}", idx + 1, lines[idx]))
                    .collect();
                
                snippets.push(snippet_lines.join("\n"));
                
                // Limit snippets to prevent overwhelming output
                if snippets.len() >= 5 {
                    break;
                }
            }
        }
        
        snippets
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
        "Sage's context engine, the world's best codebase context engine. It takes in a natural language description of the code you are looking for and uses a proprietary retrieval/embedding model suite that produces the highest-quality recall of relevant code snippets from across the codebase."
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
                        "description": "A description of the information you need."
                    }
                },
                "required": ["information_request"]
            }),
        }
    }
}
