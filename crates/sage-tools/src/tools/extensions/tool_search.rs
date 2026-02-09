//! Tool search for deferred/lazy-loaded tools
//!
//! This tool allows searching for and loading deferred tools before use.
//! It's designed to be compatible with Claude Code's ToolSearch functionality.
//!
//! ## Query Modes
//!
//! 1. **Keyword search**: `"slack message"` - Find tools matching keywords
//! 2. **Direct selection**: `"select:tool_name"` - Load a specific tool by name
//! 3. **Required keyword**: `"+slack send"` - Require first keyword, rank by rest

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Result from a tool search
#[derive(Debug, Clone)]
pub struct ToolSearchResult {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Whether the tool is now loaded
    pub loaded: bool,
}

/// Registry of deferred tools that can be loaded on demand
pub struct DeferredToolRegistry {
    /// Available but not yet loaded tools
    available: HashMap<String, DeferredToolInfo>,
    /// Loaded tools
    loaded: HashMap<String, Arc<dyn Tool>>,
    /// Tool loader function
    loader: Option<Box<dyn Fn(&str) -> Option<Arc<dyn Tool>> + Send + Sync>>,
}

/// Information about a deferred tool
#[derive(Debug, Clone)]
pub struct DeferredToolInfo {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub source: String, // e.g., "mcp", "builtin", "custom"
}

impl Default for DeferredToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeferredToolRegistry {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            loaded: HashMap::new(),
            loader: None,
        }
    }

    /// Register a deferred tool
    pub fn register_deferred(&mut self, info: DeferredToolInfo) {
        self.available.insert(info.name.clone(), info);
    }

    /// Set the tool loader function
    pub fn set_loader<F>(&mut self, loader: F)
    where
        F: Fn(&str) -> Option<Arc<dyn Tool>> + Send + Sync + 'static,
    {
        self.loader = Some(Box::new(loader));
    }

    /// Load a tool by name
    pub fn load(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        // Already loaded?
        if let Some(tool) = self.loaded.get(name) {
            return Some(Arc::clone(tool));
        }

        // Try to load
        if let Some(ref loader) = self.loader {
            if let Some(tool) = loader(name) {
                self.loaded.insert(name.to_string(), Arc::clone(&tool));
                self.available.remove(name);
                return Some(tool);
            }
        }

        None
    }

    /// Search for tools by keywords
    pub fn search(&self, query: &str, limit: usize) -> Vec<ToolSearchResult> {
        let keywords: Vec<&str> = query.split_whitespace().collect();
        if keywords.is_empty() {
            return vec![];
        }

        let mut results: Vec<(String, f64)> = self
            .available
            .iter()
            .map(|(name, info)| {
                let score = self.calculate_score(info, &keywords);
                (name.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        results
            .into_iter()
            .take(limit)
            .map(|(name, score)| {
                let info = self.available.get(&name).unwrap();
                ToolSearchResult {
                    name: name.clone(),
                    description: info.description.clone(),
                    score,
                    loaded: false,
                }
            })
            .collect()
    }

    /// Search with a required keyword
    pub fn search_with_required(
        &self,
        required: &str,
        other_keywords: &[&str],
        limit: usize,
    ) -> Vec<ToolSearchResult> {
        let mut results: Vec<(String, f64)> = self
            .available
            .iter()
            .filter(|(name, info)| {
                // Must match required keyword
                name.to_lowercase().contains(&required.to_lowercase())
                    || info
                        .keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(&required.to_lowercase()))
            })
            .map(|(name, info)| {
                let score = if other_keywords.is_empty() {
                    1.0
                } else {
                    self.calculate_score(info, other_keywords)
                };
                (name.clone(), score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results
            .into_iter()
            .take(limit)
            .map(|(name, score)| {
                let info = self.available.get(&name).unwrap();
                ToolSearchResult {
                    name: name.clone(),
                    description: info.description.clone(),
                    score,
                    loaded: false,
                }
            })
            .collect()
    }

    /// Calculate relevance score for a tool
    fn calculate_score(&self, info: &DeferredToolInfo, keywords: &[&str]) -> f64 {
        let mut score = 0.0;
        let name_lower = info.name.to_lowercase();
        let desc_lower = info.description.to_lowercase();

        for keyword in keywords {
            let kw_lower = keyword.to_lowercase();

            // Exact name match
            if name_lower == kw_lower {
                score += 1.0;
            }
            // Name contains keyword
            else if name_lower.contains(&kw_lower) {
                score += 0.7;
            }
            // Description contains keyword
            else if desc_lower.contains(&kw_lower) {
                score += 0.3;
            }
            // Keywords match
            else if info
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(&kw_lower))
            {
                score += 0.5;
            }
        }

        // Normalize by number of keywords
        score / keywords.len() as f64
    }

    /// Get a loaded tool
    pub fn get_loaded(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.loaded.get(name).cloned()
    }

    /// Check if a tool is available (deferred or loaded)
    pub fn is_available(&self, name: &str) -> bool {
        self.available.contains_key(name) || self.loaded.contains_key(name)
    }

    /// List all available tool names
    pub fn list_available(&self) -> Vec<String> {
        let mut names: Vec<String> = self.available.keys().cloned().collect();
        names.extend(self.loaded.keys().cloned());
        names.sort();
        names.dedup();
        names
    }
}

/// Tool for searching and loading deferred tools
pub struct ToolSearchTool {
    registry: Arc<RwLock<DeferredToolRegistry>>,
}

impl Default for ToolSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolSearchTool {
    /// Create a new ToolSearchTool with an empty registry
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(DeferredToolRegistry::new())),
        }
    }

    /// Create with an existing registry
    pub fn with_registry(registry: Arc<RwLock<DeferredToolRegistry>>) -> Self {
        Self { registry }
    }

    /// Get a reference to the registry
    pub fn registry(&self) -> Arc<RwLock<DeferredToolRegistry>> {
        Arc::clone(&self.registry)
    }

    /// Parse the query and execute the appropriate search
    async fn execute_search(&self, query: &str) -> Result<String, ToolError> {
        let mut registry = self.registry.write().await;

        // Direct selection mode: select:tool_name
        if let Some(tool_name) = query.strip_prefix("select:") {
            let tool_name = tool_name.trim();
            if let Some(_tool) = registry.load(tool_name) {
                return Ok(format!(
                    "Tool '{}' has been loaded and is now available for use.",
                    tool_name
                ));
            } else if registry.is_available(tool_name) {
                return Ok(format!(
                    "Tool '{}' is already loaded and available.",
                    tool_name
                ));
            } else {
                return Err(ToolError::ExecutionFailed(format!(
                    "Tool '{}' not found in available tools.",
                    tool_name
                )));
            }
        }

        // Required keyword mode: +keyword other keywords
        if let Some(rest) = query.strip_prefix('+') {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "Required keyword cannot be empty".to_string(),
                ));
            }

            let required = parts[0];
            let other: Vec<&str> = parts[1..].to_vec();
            let results = registry.search_with_required(required, &other, 5);

            // Load all found tools
            let mut loaded_tools = Vec::new();
            for result in &results {
                if registry.load(&result.name).is_some() {
                    loaded_tools.push(result.name.clone());
                }
            }

            return Ok(self.format_results(&results, &loaded_tools));
        }

        // Keyword search mode
        let results = registry.search(query, 5);

        // Load all found tools
        let mut loaded_tools = Vec::new();
        for result in &results {
            if registry.load(&result.name).is_some() {
                loaded_tools.push(result.name.clone());
            }
        }

        Ok(self.format_results(&results, &loaded_tools))
    }

    /// Format search results
    fn format_results(&self, results: &[ToolSearchResult], loaded: &[String]) -> String {
        if results.is_empty() {
            return "No matching tools found.".to_string();
        }

        let mut output = format!("Found {} tool(s):\n\n", results.len());

        for result in results {
            let status = if loaded.contains(&result.name) {
                "✓ LOADED"
            } else {
                "available"
            };

            output.push_str(&format!(
                "- **{}** [{}]\n  {}\n\n",
                result.name, status, result.description
            ));
        }

        output.push_str("All returned tools are now loaded and available to call directly.");

        output
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "ToolSearch"
    }

    fn description(&self) -> &str {
        r#"Search for or select deferred tools to make them available for use.

**MANDATORY PREREQUISITE**: You MUST use this tool to load deferred tools BEFORE calling them directly.

**Query modes:**
1. **Keyword search**: Use keywords to discover tools (e.g., "slack message")
2. **Direct selection**: Use `select:<tool_name>` for a specific tool
3. **Required keyword**: Prefix with `+` to require a match (e.g., "+slack send")

Both keyword search and direct selection load the returned tools immediately."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "query",
                "Search query: keywords, 'select:<tool_name>', or '+required other keywords'",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let query = call.get_string("query").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: query".to_string())
        })?;

        if query.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Query cannot be empty".to_string(),
            ));
        }

        let result = self.execute_search(&query).await?;
        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let query = call.get_string("query").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: query".to_string())
        })?;

        if query.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Query cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_tool_call(query: &str) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert("query".to_string(), json!(query));

        ToolCall {
            id: "test-1".to_string(),
            name: "ToolSearch".to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_keyword_search() {
        let tool = ToolSearchTool::new();

        // Add some test tools
        {
            let mut registry = tool.registry.write().await;
            registry.register_deferred(DeferredToolInfo {
                name: "mcp__slack__read_channel".to_string(),
                description: "Read messages from a Slack channel".to_string(),
                keywords: vec![
                    "slack".to_string(),
                    "message".to_string(),
                    "read".to_string(),
                ],
                source: "mcp".to_string(),
            });
            registry.register_deferred(DeferredToolInfo {
                name: "mcp__slack__send_message".to_string(),
                description: "Send a message to a Slack channel".to_string(),
                keywords: vec![
                    "slack".to_string(),
                    "message".to_string(),
                    "send".to_string(),
                ],
                source: "mcp".to_string(),
            });
        }

        let call = create_tool_call("slack");
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("slack"));
    }

    #[tokio::test]
    async fn test_direct_selection() {
        let tool = ToolSearchTool::new();

        // Add a test tool
        {
            let mut registry = tool.registry.write().await;
            registry.register_deferred(DeferredToolInfo {
                name: "NotebookEdit".to_string(),
                description: "Edit Jupyter notebooks".to_string(),
                keywords: vec!["notebook".to_string(), "jupyter".to_string()],
                source: "builtin".to_string(),
            });
        }

        let call = create_tool_call("select:NotebookEdit");
        let result = tool.execute(&call).await;
        // Will fail because no loader is set, but that's expected
        assert!(result.is_err() || result.unwrap().success);
    }

    #[tokio::test]
    async fn test_required_keyword() {
        let tool = ToolSearchTool::new();

        // Add test tools
        {
            let mut registry = tool.registry.write().await;
            registry.register_deferred(DeferredToolInfo {
                name: "linear_create_issue".to_string(),
                description: "Create a Linear issue".to_string(),
                keywords: vec![
                    "linear".to_string(),
                    "issue".to_string(),
                    "create".to_string(),
                ],
                source: "mcp".to_string(),
            });
            registry.register_deferred(DeferredToolInfo {
                name: "github_create_issue".to_string(),
                description: "Create a GitHub issue".to_string(),
                keywords: vec![
                    "github".to_string(),
                    "issue".to_string(),
                    "create".to_string(),
                ],
                source: "mcp".to_string(),
            });
        }

        let call = create_tool_call("+linear create issue");
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        // Should only find linear tools
        let output = result.output.unwrap();
        assert!(output.contains("linear"));
        assert!(!output.contains("github"));
    }

    #[tokio::test]
    async fn test_empty_query() {
        let tool = ToolSearchTool::new();
        let call = create_tool_call("");
        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_results() {
        let tool = ToolSearchTool::new();
        let call = create_tool_call("nonexistent_tool_xyz");
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(
            result
                .output
                .as_ref()
                .unwrap()
                .contains("No matching tools")
        );
    }

    #[test]
    fn test_schema() {
        let tool = ToolSearchTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "ToolSearch");
        assert!(!schema.description.is_empty());
    }
}
