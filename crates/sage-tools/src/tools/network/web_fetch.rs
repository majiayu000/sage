use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::debug;

use super::redirect::{is_redirect_status, validate_redirect_target};
use super::validation::validate_url_security;

/// HTTP client for web fetching
static CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> = std::sync::OnceLock::new();
const WEB_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

fn get_client() -> anyhow::Result<&'static reqwest::Client> {
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(WEB_FETCH_TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("Sage-Agent-WebFetch/1.0")
                .build()
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| anyhow::anyhow!("Failed to create WebFetch HTTP client: {error}"))
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

    async fn fetch_response(
        client: &reqwest::Client,
        url: &str,
    ) -> anyhow::Result<reqwest::Response> {
        Self::fetch_response_with_timeout(client, url, WEB_FETCH_TIMEOUT).await
    }

    async fn fetch_response_with_timeout(
        client: &reqwest::Client,
        url: &str,
        request_timeout: Duration,
    ) -> anyhow::Result<reqwest::Response> {
        validate_url_security(url).await?;
        let current_url = reqwest::Url::parse(url).context("Invalid URL format")?;
        let response = timeout(request_timeout, client.get(current_url).send())
            .await
            .context("Request timeout")?
            .context("Failed to fetch URL")?;

        if !is_redirect_status(response.status()) {
            return Ok(response);
        }

        let next_url = validate_redirect_target(response.url(), response.headers()).await?;
        anyhow::bail!(
            "WebFetch request redirected to {}. Make a new WebFetch request with the redirect URL to fetch it.",
            next_url
        );
    }

    /// Fetch URL and convert HTML to Markdown
    async fn fetch_and_convert(&self, url: &str) -> anyhow::Result<String> {
        debug!("Fetching URL: {}", url);

        let client = get_client()?;
        let response = Self::fetch_response(client, url).await?;

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
  - HTTP URLs are fetched as requested; if the server redirects to HTTPS, make a new WebFetch request with the redirect URL
  - This tool is read-only and does not modify any files
  - Results may be summarized if the content is very large
  - Includes caching for faster responses when repeatedly accessing the same URL
  - When a URL redirects, the tool will inform you and provide the redirect URL. You should then make a new WebFetch request with the redirect URL to fetch the content.
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

        // Validate the initial URL here to keep argument errors classified
        // before the fetch path validates each redirect target.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_redirecting_server() -> anyhow::Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind WebFetch redirect test server")?;
        let addr = listener
            .local_addr()
            .context("read WebFetch redirect test server addr")?;
        let redirect_target = format!("http://127.0.0.1:{}/private", addr.port());
        let proxy_url = format!("http://{}", addr);

        tokio::spawn(async move {
            for request_index in 0..2 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };

                let mut buffer = [0_u8; 2048];
                if socket.read(&mut buffer).await.is_err() {
                    return;
                }

                let response = if request_index == 0 {
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: {redirect_target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 12\r\nConnection: close\r\n\r\nprivate body".to_string()
                };

                if socket.write_all(response.as_bytes()).await.is_err() {
                    return;
                }
            }
        });

        Ok(proxy_url)
    }

    async fn spawn_slow_web_fetch_redirect_proxy() -> anyhow::Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind slow WebFetch redirect test proxy")?;
        let addr = listener
            .local_addr()
            .context("read slow WebFetch redirect test proxy addr")?;
        let proxy_url = format!("http://{}", addr);

        tokio::spawn(async move {
            for request_index in 0..2 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };

                let mut buffer = [0_u8; 2048];
                if socket.read(&mut buffer).await.is_err() {
                    return;
                }

                tokio::time::sleep(Duration::from_millis(700)).await;
                let response = if request_index == 0 {
                    "HTTP/1.1 302 Found\r\nLocation: http://2.2.2.2/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                };

                if socket.write_all(response.as_bytes()).await.is_err() {
                    return;
                }
            }
        });

        Ok(proxy_url)
    }

    #[tokio::test]
    async fn test_web_fetch_rejects_redirect_to_loopback() -> anyhow::Result<()> {
        let proxy_url = spawn_redirecting_server().await?;
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::http(proxy_url)?)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let result = WebFetchTool::fetch_response(&client, "http://1.1.1.1/").await;

        assert!(
            result.is_err(),
            "WebFetch must reject redirects to loopback/private targets"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_web_fetch_returns_redirect_target_without_fetching_it() -> anyhow::Result<()> {
        let proxy_url = spawn_slow_web_fetch_redirect_proxy().await?;
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::http(proxy_url)?)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let start = tokio::time::Instant::now();
        let result = WebFetchTool::fetch_response_with_timeout(
            &client,
            "http://1.1.1.1/",
            Duration::from_secs(2),
        )
        .await;

        let error =
            result.expect_err("WebFetch must return redirect targets without fetching them");
        assert!(
            error.to_string().contains("http://2.2.2.2/final"),
            "redirect error should include the next URL: {error}"
        );
        assert!(
            start.elapsed() < Duration::from_millis(1300),
            "WebFetch fetched the redirect target instead of stopping at the redirect"
        );

        Ok(())
    }
}
