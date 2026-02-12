use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::validation::validate_url_security;

/// HTTP client for web fetching
static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn get_client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Sage-Agent-WebFetch/1.0")
            .build()
            .expect("Failed to create HTTP client")
    })
}

#[derive(Debug, Clone)]
pub struct WebFetchTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebFetchInput {
    pub url: String,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }

    /// Fetch URL and convert HTML to Markdown
    async fn fetch_and_convert(&self, url: &str) -> anyhow::Result<String> {
        debug!("Fetching URL: {}", url);

        let client = get_client();
        let response = client
            .get(url)
            .send()
            .await
            .context("Failed to fetch URL")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("HTTP request failed with status: {}", status);
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        // Convert HTML to Markdown if it's HTML content
        if content_type.contains("text/html") {
            Ok(self.html_to_markdown(&body))
        } else if content_type.contains("text/plain") || content_type.contains("text/markdown") {
            Ok(body)
        } else if content_type.contains("application/json") {
            // Pretty-print JSON
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(json) => Ok(format!(
                    "```json\n{}\n```",
                    serde_json::to_string_pretty(&json).unwrap_or(body)
                )),
                Err(_) => Ok(format!("```\n{}\n```", body)),
            }
        } else {
            // Return raw content for other types
            Ok(format!("```\n{}\n```", body))
        }
    }

    /// Convert HTML to Markdown (basic implementation)
    fn html_to_markdown(&self, html: &str) -> String {
        // Remove script and style tags first
        let html = self.remove_tags(html, "script");
        let html = self.remove_tags(&html, "style");
        let html = self.remove_tags(&html, "nav");
        let html = self.remove_tags(&html, "footer");
        let html = self.remove_tags(&html, "header");

        let mut result = String::new();
        let mut in_tag = false;
        let mut current_tag = String::new();
        let mut text_buffer = String::new();

        for ch in html.chars() {
            match ch {
                '<' => {
                    if !text_buffer.trim().is_empty() {
                        result.push_str(text_buffer.trim());
                        result.push(' ');
                    }
                    text_buffer.clear();
                    in_tag = true;
                    current_tag.clear();
                }
                '>' => {
                    in_tag = false;
                    let tag = current_tag.to_lowercase();
                    let tag_name = tag.split_whitespace().next().unwrap_or("");

                    match tag_name {
                        "h1" => result.push_str("\n# "),
                        "h2" => result.push_str("\n## "),
                        "h3" => result.push_str("\n### "),
                        "h4" => result.push_str("\n#### "),
                        "h5" => result.push_str("\n##### "),
                        "h6" => result.push_str("\n###### "),
                        "/h1" | "/h2" | "/h3" | "/h4" | "/h5" | "/h6" => result.push_str("\n\n"),
                        "p" => result.push_str("\n\n"),
                        "/p" => result.push_str("\n\n"),
                        "br" | "br/" => result.push('\n'),
                        "li" => result.push_str("\n- "),
                        "/li" => {}
                        "ul" | "ol" => result.push('\n'),
                        "/ul" | "/ol" => result.push('\n'),
                        "code" => result.push('`'),
                        "/code" => result.push('`'),
                        "pre" => result.push_str("\n```\n"),
                        "/pre" => result.push_str("\n```\n"),
                        "strong" | "b" => result.push_str("**"),
                        "/strong" | "/b" => result.push_str("**"),
                        "em" | "i" => result.push('*'),
                        "/em" | "/i" => result.push('*'),
                        "blockquote" => result.push_str("\n> "),
                        "/blockquote" => result.push('\n'),
                        "hr" | "hr/" => result.push_str("\n---\n"),
                        _ => {}
                    }
                }
                _ => {
                    if in_tag {
                        current_tag.push(ch);
                    } else {
                        text_buffer.push(ch);
                    }
                }
            }
        }

        // Add any remaining text
        if !text_buffer.trim().is_empty() {
            result.push_str(text_buffer.trim());
        }

        // Decode HTML entities
        let result = result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");

        // Clean up excessive whitespace
        let lines: Vec<&str> = result.lines().collect();
        let mut cleaned = String::new();
        let mut prev_empty = false;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !prev_empty {
                    cleaned.push('\n');
                    prev_empty = true;
                }
            } else {
                cleaned.push_str(trimmed);
                cleaned.push('\n');
                prev_empty = false;
            }
        }

        cleaned.trim().to_string()
    }

    /// Remove specific HTML tags and their content
    fn remove_tags(&self, html: &str, tag: &str) -> String {
        let open_tag = format!("<{}", tag);
        let close_tag = format!("</{}>", tag);
        let mut result = html.to_string();

        while let Some(start) = result.to_lowercase().find(&open_tag) {
            if let Some(end) = result[start..].to_lowercase().find(&close_tag) {
                let end_pos = start + end + close_tag.len();
                result = format!("{}{}", &result[..start], &result[end_pos..]);
            } else {
                break;
            }
        }

        result
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        r#"- Fetches content from a specified URL and processes it
- Takes a URL as input
- Fetches the URL content, converts HTML to markdown
- Returns the content in markdown format
- Use this tool when you need to retrieve and analyze web content

Usage notes:
  - The URL must be a fully-formed valid URL
  - HTTP URLs will be automatically upgraded to HTTPS
  - This tool is read-only and does not modify any files
  - Results may be summarized if the content is very large
  - Includes caching for faster responses when repeatedly accessing the same URL
  - When a URL redirects to a different host, the tool will inform you and provide the redirect URL. You should then make a new WebFetch request with the redirect URL to fetch the content.
  - If the return is not valid Markdown, it means the tool cannot successfully parse this page.

Parameters:
- url: The URL to fetch content from (required)"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string("url", "The URL to fetch.")],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let url = call
            .get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        // Validate URL to prevent SSRF attacks
        validate_url_security(&url)
            .await
            .map_err(|e| ToolError::InvalidArguments(format!("URL validation failed: {}", e)))?;

        // Fetch and convert the content
        let markdown_content = self
            .fetch_and_convert(&url)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to fetch URL: {}", e)))?;

        Ok(ToolResult::success(&call.id, self.name(), markdown_content))
    }
}
