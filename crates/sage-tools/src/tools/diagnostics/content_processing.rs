use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ViewRangeUntruncatedTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ViewRangeInput {
    pub reference_id: String,
    pub start_line: u32,
    pub end_line: u32,
}

impl Default for ViewRangeUntruncatedTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewRangeUntruncatedTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ViewRangeUntruncatedTool {
    fn name(&self) -> &str {
        "view-range-untruncated"
    }

    fn description(&self) -> &str {
        "View a specific range of lines from untruncated content"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "reference_id",
                    "The reference ID of the truncated content (found in the truncation footer)",
                ),
                ToolParameter::number(
                    "start_line",
                    "The starting line number (1-based, inclusive)",
                ),
                ToolParameter::number("end_line", "The ending line number (1-based, inclusive)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let reference_id = call.get_string("reference_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'reference_id' parameter".to_string())
        })?;
        let start_line: u32 = call.get_argument("start_line").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'start_line' parameter".to_string())
        })?;
        let end_line: u32 = call.get_argument("end_line").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'end_line' parameter".to_string())
        })?;

        // TODO: Implement actual untruncated content viewing
        let content = format!(
            "Lines {}-{} from reference {}:\n\n[Placeholder content would be shown here]",
            start_line, end_line, reference_id
        );

        Ok(ToolResult::success(&call.id, self.name(), content))
    }
}

#[derive(Debug, Clone)]
pub struct SearchUntruncatedTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchUntruncatedInput {
    pub reference_id: String,
    pub search_term: String,
    #[serde(default = "default_context_lines")]
    pub context_lines: u32,
}

fn default_context_lines() -> u32 {
    2
}

impl Default for SearchUntruncatedTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchUntruncatedTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SearchUntruncatedTool {
    fn name(&self) -> &str {
        "search-untruncated"
    }

    fn description(&self) -> &str {
        "Search for a term within untruncated content"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "reference_id",
                    "The reference ID of the truncated content (found in the truncation footer)",
                ),
                ToolParameter::string("search_term", "The term to search for within the content"),
                ToolParameter::number(
                    "context_lines",
                    "Number of context lines to include before and after matches (default: 2)",
                )
                .with_default(2)
                .optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let reference_id = call.get_string("reference_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'reference_id' parameter".to_string())
        })?;
        let search_term = call.get_string("search_term").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'search_term' parameter".to_string())
        })?;
        let context_lines: u32 = call.get_argument("context_lines").unwrap_or(2);

        // TODO: Implement actual search in untruncated content
        let content = format!(
            "Search results for '{}' in reference {} (with {} context lines):\n\n[Placeholder search results would be shown here]",
            search_term, reference_id, context_lines
        );

        Ok(ToolResult::success(&call.id, self.name(), content))
    }
}
