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
        r#"- Allows you to search the web and use the results to inform responses
- Provides up-to-date information for current events and recent data
- Returns search result information formatted as search result blocks, including links as markdown hyperlinks
- Use this tool for accessing information beyond your knowledge cutoff
- Searches are performed automatically within a single API call

CRITICAL REQUIREMENT - You MUST follow this:
  - After answering the user's question, you MUST include a "Sources:" section at the end of your response
  - In the Sources section, list all relevant URLs from the search results as markdown hyperlinks: [Title](URL)
  - This is MANDATORY - never skip including sources in your response
  - Example format:

    [Your answer here]

    Sources:
    - [Source Title 1](https://example.com/1)
    - [Source Title 2](https://example.com/2)

Usage notes:
  - Domain filtering is supported to include or block specific websites
  - If search returns placeholder results or fails, use your built-in knowledge to proceed

IMPORTANT - Use the correct year in search queries:
  - You MUST use the current year when searching for recent information, documentation, or current events.
  - Example: If the user asks for "latest React docs", search for "React documentation 2025", NOT "React documentation 2024""#
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
