use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct WebSearchTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchInput {
    pub query: String,
    #[serde(default = "default_num_results")]
    pub num_results: u32,
}

fn default_num_results() -> u32 {
    5
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchOutput {
    pub results: Vec<SearchResult>,
    pub query: String,
    pub total_results: u32,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web-search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns results in markdown format.\n\
         IMPORTANT: If search returns placeholder results or fails, DO NOT retry indefinitely.\n\
         Instead, use your built-in knowledge to proceed with the task.\n\
         Your training data contains extensive information about common APIs, frameworks, and programming topics.\n\
         Proceed with implementation using your knowledge rather than waiting for search results."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("query", "The search query to send."),
                ToolParameter::number("num_results", "Number of results to return")
                    .with_default(5)
                    .optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let query = call
            .get_string("query")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".to_string()))?;

        let _num_results = call.get_argument::<u32>("num_results").unwrap_or(5);

        // NOTE: This returns placeholder results to guide the agent to use its built-in knowledge.
        // Actual web search requires external API integration (e.g., Google Search API, Bing API).
        // To implement real search:
        // 1. Add API credentials to configuration
        // 2. Integrate with search provider (reqwest + API endpoints)
        // 3. Parse and format results as markdown with links
        let markdown = format!(
            r#"# Web Search: {}

⚠️ **NOTE**: Web search is currently returning placeholder results.

**DO NOT retry the search or wait for better results.**

Instead, please proceed with your task using your built-in knowledge:
- You have extensive knowledge about common APIs (weather APIs, REST APIs, etc.)
- You know about popular frameworks and their usage
- You can implement features based on standard patterns

**ACTION REQUIRED**: Start writing code NOW using your training knowledge.
For weather APIs, you know about: OpenWeatherMap, WeatherAPI, Open-Meteo (free, no API key).

REMEMBER: Your job is to WRITE CODE, not to search endlessly.
"#,
            query
        );

        Ok(ToolResult::success(&call.id, self.name(), markdown))
    }
}
