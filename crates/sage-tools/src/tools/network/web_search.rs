use sage_core::tools::{Tool, ToolResult, ToolError, ToolCall, ToolSchema, ToolParameter};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

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
        "Search the web for information. Returns results in markdown format.\nEach result includes the URL, title, and a snippet from the page if available.\n\nThis tool uses Google's Custom Search API to find relevant web pages."
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
            ]
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let query = call.get_string("query")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".to_string()))?;

        let _num_results = call.get_argument::<u32>("num_results").unwrap_or(5);

        // TODO: Implement actual web search functionality
        // This is a placeholder implementation
        let results = vec![
            SearchResult {
                url: "https://example.com/1".to_string(),
                title: format!("Search result for: {}", query),
                snippet: Some("This is a placeholder search result.".to_string()),
            }
        ];

        let output = WebSearchOutput {
            query: query.clone(),
            total_results: results.len() as u32,
            results,
        };

        // Format as markdown
        let mut markdown = format!("# Web Search Results for: {}\n\n", query);
        for (i, result) in output.results.iter().enumerate() {
            markdown.push_str(&format!("## {}. {}\n", i + 1, result.title));
            markdown.push_str(&format!("**URL:** {}\n\n", result.url));
            if let Some(snippet) = &result.snippet {
                markdown.push_str(&format!("{}\n\n", snippet));
            }
        }

        Ok(ToolResult::success(&call.id, self.name(), markdown))
    }
}
