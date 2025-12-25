//! HTTP client tool implementation

use async_trait::async_trait;
use tracing::info;

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

use super::request::{create_client, execute_request, format_response};
use super::types::{HttpClientParams, HttpMethod};

/// HTTP client tool for making HTTP requests
#[derive(Debug, Clone)]
pub struct HttpClientTool {
    name: String,
    description: String,
    client: Option<reqwest::Client>,
}

impl HttpClientTool {
    /// Create a new HTTP client tool
    pub fn new() -> Self {
        Self {
            name: "http_client".to_string(),
            description: "HTTP client for REST API interactions, GraphQL queries, and web requests"
                .to_string(),
            client: None,
        }
    }

    /// Get or create HTTP client
    fn get_or_create_client(
        &mut self,
        verify_ssl: bool,
        follow_redirects: bool,
        timeout_secs: u64,
    ) -> Result<&reqwest::Client, ToolError> {
        if self.client.is_none() {
            let client = create_client(verify_ssl, follow_redirects, timeout_secs)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            self.client = Some(client);
        }
        // SAFETY: client is guaranteed to be Some after the above initialization
        self.client
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("Client initialization failed".to_string()))
    }
}

impl Default for HttpClientTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpClientTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("method", "HTTP method").with_default("GET".to_string()),
                ToolParameter::string("url", "Request URL"),
                ToolParameter::optional_string("headers", "Request headers as JSON object string"),
                ToolParameter::optional_string("body", "Request body as JSON string"),
                ToolParameter::optional_string("auth", "Authentication config as JSON string"),
                ToolParameter::number("timeout", "Request timeout in seconds")
                    .optional()
                    .with_default(30),
                ToolParameter::boolean("follow_redirects", "Follow redirects")
                    .optional()
                    .with_default(true),
                ToolParameter::boolean("verify_ssl", "Verify SSL certificates")
                    .optional()
                    .with_default(true),
                ToolParameter::optional_string("save_to_file", "Save response to file"),
                ToolParameter::optional_string("graphql_query", "GraphQL query string"),
                ToolParameter::optional_string(
                    "graphql_variables",
                    "GraphQL variables as JSON string",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let method_str = call
            .get_string("method")
            .unwrap_or_else(|| "GET".to_string());
        let method = match method_str.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid HTTP method: {}",
                    method_str
                )));
            }
        };

        let url = call
            .get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        let headers = call
            .get_string("headers")
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid headers JSON: {}", e)))?;

        let body = call
            .get_string("body")
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid body JSON: {}", e)))?;

        let auth = call
            .get_string("auth")
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid auth JSON: {}", e)))?;

        let graphql_variables = call
            .get_string("graphql_variables")
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| {
                ToolError::InvalidArguments(format!("Invalid graphql_variables JSON: {}", e))
            })?;

        let params = HttpClientParams {
            method,
            url: url.clone(),
            headers,
            body,
            auth,
            timeout: call.get_number("timeout").map(|n| n as u64),
            follow_redirects: call.get_bool("follow_redirects"),
            verify_ssl: call.get_bool("verify_ssl"),
            save_to_file: call.get_string("save_to_file"),
            graphql_query: call.get_string("graphql_query"),
            graphql_variables,
        };

        info!("Executing HTTP request: {:?} {}", params.method, params.url);

        let mut tool = self.clone();
        let verify_ssl = params.verify_ssl.unwrap_or(true);
        let follow_redirects = params.follow_redirects.unwrap_or(true);
        let timeout_secs = params.timeout.unwrap_or(30);

        let client = tool.get_or_create_client(verify_ssl, follow_redirects, timeout_secs)?;
        let response = execute_request(client, params)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        let output = format_response(&response);

        let mut result = ToolResult::success(&call.id, self.name(), output)
            .with_metadata("status", serde_json::Value::Number(response.status.into()))
            .with_metadata(
                "response_time_ms",
                serde_json::Value::Number(response.response_time.into()),
            )
            .with_metadata("url", serde_json::Value::String(url));

        if let Some(content_type) = response.content_type {
            result = result.with_metadata("content_type", serde_json::Value::String(content_type));
        }

        if let Some(content_length) = response.content_length {
            result = result.with_metadata(
                "content_length",
                serde_json::Value::Number(content_length.into()),
            );
        }

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        call.get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        if let Some(method_str) = call.get_string("method") {
            let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
            if !valid_methods.contains(&method_str.to_uppercase().as_str()) {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid HTTP method: {}. Must be one of: {}",
                    method_str,
                    valid_methods.join(", ")
                )));
            }
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(300)
    }

    fn supports_parallel_execution(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_tool_creation() {
        let tool = HttpClientTool::new();
        assert_eq!(tool.name(), "http_client");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_http_client_schema() {
        let tool = HttpClientTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "http_client");
        assert!(!schema.description.is_empty());
        assert!(schema.parameters.is_object());
    }
}
