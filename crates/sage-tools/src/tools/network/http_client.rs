//! HTTP Client Tool
//!
//! This tool provides HTTP client functionality including:
//! - REST API interactions
//! - GraphQL support
//! - Custom headers and authentication
//! - Request/response processing
//! - File uploads and downloads

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context, anyhow};
use tokio::time::timeout;
use tracing::{info, debug, error};

use sage_core::tools::{Tool, ToolResult};

/// HTTP methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

/// HTTP request body types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestBody {
    /// JSON body
    Json(serde_json::Value),
    /// Plain text body
    Text(String),
    /// Form data
    Form(HashMap<String, String>),
    /// Raw binary data (base64 encoded)
    Binary(String),
}

/// HTTP authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    /// Bearer token
    Bearer { token: String },
    /// Basic authentication
    Basic { username: String, password: String },
    /// API key in header
    ApiKey { key: String, value: String },
}

/// HTTP client tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientParams {
    /// HTTP method
    pub method: HttpMethod,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: Option<HashMap<String, String>>,
    /// Request body
    pub body: Option<RequestBody>,
    /// Authentication
    pub auth: Option<AuthType>,
    /// Request timeout in seconds
    pub timeout: Option<u64>,
    /// Follow redirects
    pub follow_redirects: Option<bool>,
    /// Verify SSL certificates
    pub verify_ssl: Option<bool>,
    /// Save response to file
    pub save_to_file: Option<String>,
    /// GraphQL specific parameters
    pub graphql_query: Option<String>,
    pub graphql_variables: Option<serde_json::Value>,
}

/// HTTP response information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Response time in milliseconds
    pub response_time: u64,
    /// Content type
    pub content_type: Option<String>,
    /// Content length
    pub content_length: Option<u64>,
}

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
            description: "HTTP client for REST API interactions, GraphQL queries, and web requests".to_string(),
            client: None,
        }
    }

    /// Get or create HTTP client
    fn get_client(&mut self, verify_ssl: bool, follow_redirects: bool, timeout_secs: u64) -> Result<&reqwest::Client> {
        if self.client.is_none() {
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(!verify_ssl)
                .redirect(if follow_redirects {
                    reqwest::redirect::Policy::limited(10)
                } else {
                    reqwest::redirect::Policy::none()
                })
                .timeout(Duration::from_secs(timeout_secs))
                .user_agent("Sage-Agent-HTTP-Client/1.0")
                .build()
                .context("Failed to create HTTP client")?;

            self.client = Some(client);
        }

        // SAFETY: unwrap is safe here because we just ensured self.client is Some above
        Ok(self.client.as_ref().expect("client should be initialized"))
    }

    /// Convert HTTP method to reqwest method
    fn to_reqwest_method(&self, method: &HttpMethod) -> reqwest::Method {
        match method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
        }
    }

    /// Build request with authentication
    fn add_auth(&self, mut request: reqwest::RequestBuilder, auth: &AuthType) -> reqwest::RequestBuilder {
        match auth {
            AuthType::Bearer { token } => {
                request = request.header("Authorization", format!("Bearer {}", token));
            }
            AuthType::Basic { username, password } => {
                request = request.basic_auth(username, Some(password));
            }
            AuthType::ApiKey { key, value } => {
                request = request.header(key, value);
            }
        }
        request
    }

    /// Add request body
    fn add_body(&self, mut request: reqwest::RequestBuilder, body: &RequestBody) -> Result<reqwest::RequestBuilder> {
        match body {
            RequestBody::Json(json) => {
                request = request.json(json);
            }
            RequestBody::Text(text) => {
                request = request.body(text.clone());
            }
            RequestBody::Form(form) => {
                request = request.form(form);
            }
            RequestBody::Binary(data) => {
                let bytes = base64::decode(data)
                    .context("Failed to decode base64 binary data")?;
                request = request.body(bytes);
            }
        }
        Ok(request)
    }

    /// Create GraphQL request
    fn create_graphql_request(&self, query: &str, variables: Option<&serde_json::Value>) -> serde_json::Value {
        let mut graphql_body = serde_json::json!({
            "query": query
        });
        
        if let Some(vars) = variables {
            graphql_body["variables"] = vars.clone();
        }
        
        graphql_body
    }

    /// Execute HTTP request
    async fn execute_request(&mut self, params: HttpClientParams) -> Result<HttpResponse> {
        let timeout_secs = params.timeout.unwrap_or(30);
        let follow_redirects = params.follow_redirects.unwrap_or(true);
        let verify_ssl = params.verify_ssl.unwrap_or(true);
        
        let client = self.get_client(verify_ssl, follow_redirects, timeout_secs)?;
        let method = self.to_reqwest_method(&params.method);
        
        debug!("Making HTTP request: {} {}", method, params.url);
        
        let mut request = client.request(method, &params.url);
        
        // Add headers
        if let Some(headers) = &params.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }
        
        // Add authentication
        if let Some(auth) = &params.auth {
            request = self.add_auth(request, auth);
        }
        
        // Handle GraphQL request
        if let Some(query) = &params.graphql_query {
            let graphql_body = self.create_graphql_request(query, params.graphql_variables.as_ref());
            request = request.json(&graphql_body);
        } else if let Some(body) = &params.body {
            // Add regular request body
            request = self.add_body(request, body)?;
        }
        
        let start_time = std::time::Instant::now();
        
        // Execute request with timeout
        let response = timeout(Duration::from_secs(timeout_secs), request.send())
            .await
            .context("Request timeout")?
            .context("HTTP request failed")?;
        
        let response_time = start_time.elapsed().as_millis() as u64;
        let status = response.status().as_u16();
        
        // Extract headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(key.to_string(), value_str.to_string());
            }
        }
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        let content_length = response.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());
        
        let body = response.text().await
            .context("Failed to read response body")?;
        
        // Save to file if requested
        if let Some(file_path) = &params.save_to_file {
            tokio::fs::write(file_path, &body).await
                .context("Failed to save response to file")?;
            info!("Response saved to: {}", file_path);
        }
        
        Ok(HttpResponse {
            status,
            headers,
            body,
            response_time,
            content_type,
            content_length,
        })
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

    fn parameters_json_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                    "description": "HTTP method"
                },
                "url": {
                    "type": "string",
                    "description": "Request URL"
                },
                "headers": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Request headers"
                },
                "body": {
                    "type": "object",
                    "oneOf": [
                        {
                            "properties": {
                                "json": { "type": "object" }
                            },
                            "required": ["json"]
                        },
                        {
                            "properties": {
                                "text": { "type": "string" }
                            },
                            "required": ["text"]
                        },
                        {
                            "properties": {
                                "form": {
                                    "type": "object",
                                    "additionalProperties": { "type": "string" }
                                }
                            },
                            "required": ["form"]
                        },
                        {
                            "properties": {
                                "binary": { "type": "string" }
                            },
                            "required": ["binary"]
                        }
                    ],
                    "description": "Request body"
                },
                "auth": {
                    "type": "object",
                    "oneOf": [
                        {
                            "properties": {
                                "bearer": {
                                    "type": "object",
                                    "properties": {
                                        "token": { "type": "string" }
                                    },
                                    "required": ["token"]
                                }
                            },
                            "required": ["bearer"]
                        },
                        {
                            "properties": {
                                "basic": {
                                    "type": "object",
                                    "properties": {
                                        "username": { "type": "string" },
                                        "password": { "type": "string" }
                                    },
                                    "required": ["username", "password"]
                                }
                            },
                            "required": ["basic"]
                        },
                        {
                            "properties": {
                                "api_key": {
                                    "type": "object",
                                    "properties": {
                                        "key": { "type": "string" },
                                        "value": { "type": "string" }
                                    },
                                    "required": ["key", "value"]
                                }
                            },
                            "required": ["api_key"]
                        }
                    ],
                    "description": "Authentication"
                },
                "timeout": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 300,
                    "default": 30,
                    "description": "Request timeout in seconds"
                },
                "follow_redirects": {
                    "type": "boolean",
                    "default": true,
                    "description": "Follow redirects"
                },
                "verify_ssl": {
                    "type": "boolean",
                    "default": true,
                    "description": "Verify SSL certificates"
                },
                "save_to_file": {
                    "type": "string",
                    "description": "Save response to file"
                },
                "graphql_query": {
                    "type": "string",
                    "description": "GraphQL query string"
                },
                "graphql_variables": {
                    "type": "object",
                    "description": "GraphQL variables"
                }
            },
            "required": ["method", "url"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let mut tool = self.clone();
        let params: HttpClientParams = serde_json::from_value(params)
            .context("Failed to parse HTTP client parameters")?;

        info!("Executing HTTP request: {} {}", params.method, params.url);

        let response = tool.execute_request(params).await?;
        
        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), response.status.to_string());
        metadata.insert("response_time".to_string(), format!("{}ms", response.response_time));
        
        if let Some(content_type) = &response.content_type {
            metadata.insert("content_type".to_string(), content_type.clone());
        }
        
        if let Some(content_length) = response.content_length {
            metadata.insert("content_length".to_string(), content_length.to_string());
        }

        // Format result
        let mut result = format!("HTTP {} - Status: {}\n", response.status, response.status);
        result.push_str(&format!("Response time: {}ms\n", response.response_time));
        
        if let Some(content_type) = &response.content_type {
            result.push_str(&format!("Content-Type: {}\n", content_type));
        }
        
        if let Some(content_length) = response.content_length {
            result.push_str(&format!("Content-Length: {}\n", content_length));
        }
        
        result.push_str("\nResponse Headers:\n");
        for (key, value) in &response.headers {
            result.push_str(&format!("  {}: {}\n", key, value));
        }
        
        result.push_str("\nResponse Body:\n");
        
        // Pretty print JSON if possible
        if let Some(content_type) = &response.content_type {
            if content_type.contains("application/json") {
                match serde_json::from_str::<serde_json::Value>(&response.body) {
                    Ok(json) => {
                        result.push_str(&serde_json::to_string_pretty(&json).unwrap_or(response.body));
                    }
                    Err(_) => {
                        result.push_str(&response.body);
                    }
                }
            } else {
                result.push_str(&response.body);
            }
        } else {
            result.push_str(&response.body);
        }

        Ok(ToolResult::new(result, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_http_client_tool_creation() {
        let tool = HttpClientTool::new();
        assert_eq!(tool.name(), "http_client");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_graphql_request_creation() {
        let tool = HttpClientTool::new();
        let query = "query { user { name } }";
        let variables = Some(json!({ "id": 1 }));
        
        let request = tool.create_graphql_request(query, variables.as_ref());
        
        assert_eq!(request["query"], query);
        assert_eq!(request["variables"], variables.unwrap());
    }

    #[tokio::test]
    async fn test_http_client_schema() {
        let tool = HttpClientTool::new();
        let schema = tool.parameters_json_schema();
        
        assert!(schema.is_object());
        assert!(schema["properties"]["method"].is_object());
        assert!(schema["properties"]["url"].is_object());
    }
}