//! HTTP Client Tool
//!
//! This tool provides HTTP client functionality including:
//! - REST API interactions
//! - GraphQL support
//! - Custom headers and authentication
//! - Request/response processing
//! - File uploads and downloads

use std::collections::HashMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context, anyhow};
use tokio::time::timeout;
use tracing::{info, debug};
use url::Url;

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Validate URL to prevent SSRF attacks
fn validate_url_security(url_str: &str) -> Result<()> {
    // Parse the URL
    let url = Url::parse(url_str)
        .map_err(|e| anyhow!("Invalid URL format: {}", e))?;

    // Only allow http and https schemes
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(anyhow!(
                "URL scheme '{}' not allowed. Only http and https are permitted.",
                scheme
            ));
        }
    }

    // Get the host
    let host = url.host_str()
        .ok_or_else(|| anyhow!("URL must have a host"))?;

    // Block localhost variants
    let host_lower = host.to_lowercase();
    if host_lower == "localhost" || host_lower == "127.0.0.1" || host_lower == "::1" {
        return Err(anyhow!(
            "Requests to localhost are not allowed for security reasons"
        ));
    }

    // Block common internal hostnames
    if host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
        || host_lower.ends_with(".localhost")
    {
        return Err(anyhow!(
            "Requests to internal hostnames ({}) are not allowed",
            host
        ));
    }

    // Try to resolve the hostname and check if it's a private IP
    if let Ok(addrs) = format!("{}:80", host).to_socket_addrs() {
        for addr in addrs {
            if is_private_ip(&addr.ip()) {
                return Err(anyhow!(
                    "Requests to private/internal IP addresses are not allowed (resolved to {})",
                    addr.ip()
                ));
            }
        }
    }

    // Block AWS/cloud metadata endpoints
    if host == "169.254.169.254" || host == "metadata.google.internal" {
        return Err(anyhow!(
            "Requests to cloud metadata endpoints are not allowed"
        ));
    }

    Ok(())
}

/// Check if an IP address is private/internal
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            // 127.0.0.0/8 - Loopback
            if ipv4.is_loopback() {
                return true;
            }
            // 10.0.0.0/8 - Private
            if ipv4.octets()[0] == 10 {
                return true;
            }
            // 172.16.0.0/12 - Private
            if ipv4.octets()[0] == 172 && (ipv4.octets()[1] >= 16 && ipv4.octets()[1] <= 31) {
                return true;
            }
            // 192.168.0.0/16 - Private
            if ipv4.octets()[0] == 192 && ipv4.octets()[1] == 168 {
                return true;
            }
            // 169.254.0.0/16 - Link-local (includes cloud metadata)
            if ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254 {
                return true;
            }
            // 0.0.0.0/8 - Current network
            if ipv4.octets()[0] == 0 {
                return true;
            }
            false
        }
        IpAddr::V6(ipv6) => {
            // ::1 - Loopback
            ipv6.is_loopback()
            // fe80::/10 - Link-local
            || (ipv6.segments()[0] & 0xffc0) == 0xfe80
            // fc00::/7 - Unique local
            || (ipv6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

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
        // Validate URL to prevent SSRF attacks
        validate_url_security(&params.url)?;

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

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("method", "HTTP method")
                    .with_default("GET".to_string()),
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
                ToolParameter::optional_string("graphql_variables", "GraphQL variables as JSON string"),
            ],
        )
    }

    // Legacy method for backwards compatibility
    fn parameters_json_schema_legacy(&self) -> serde_json::Value {
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

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Parse method
        let method_str = call.get_string("method").unwrap_or_else(|| "GET".to_string());
        let method = match method_str.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            _ => return Err(ToolError::InvalidArguments(format!("Invalid HTTP method: {}", method_str))),
        };

        // Get URL (required)
        let url = call.get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        // Parse optional parameters
        let headers = if let Some(headers_str) = call.get_string("headers") {
            serde_json::from_str(&headers_str)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid headers JSON: {}", e)))?
        } else {
            None
        };

        let body = if let Some(body_str) = call.get_string("body") {
            serde_json::from_str(&body_str)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid body JSON: {}", e)))?
        } else {
            None
        };

        let auth = if let Some(auth_str) = call.get_string("auth") {
            serde_json::from_str(&auth_str)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid auth JSON: {}", e)))?
        } else {
            None
        };

        let timeout = call.get_number("timeout").map(|n| n as u64);
        let follow_redirects = call.get_bool("follow_redirects");
        let verify_ssl = call.get_bool("verify_ssl");
        let save_to_file = call.get_string("save_to_file");
        let graphql_query = call.get_string("graphql_query");

        let graphql_variables = if let Some(vars_str) = call.get_string("graphql_variables") {
            Some(serde_json::from_str(&vars_str)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid graphql_variables JSON: {}", e)))?)
        } else {
            None
        };

        let params = HttpClientParams {
            method,
            url: url.clone(),
            headers,
            body,
            auth,
            timeout,
            follow_redirects,
            verify_ssl,
            save_to_file,
            graphql_query,
            graphql_variables,
        };

        info!("Executing HTTP request: {:?} {}", params.method, params.url);

        // Execute request
        let mut tool = self.clone();
        let response = tool.execute_request(params).await
            .map_err(|e| ToolError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        // Format result
        let mut output = format!("HTTP {} - Status: {}\n", response.status, response.status);
        output.push_str(&format!("Response time: {}ms\n", response.response_time));

        if let Some(content_type) = &response.content_type {
            output.push_str(&format!("Content-Type: {}\n", content_type));
        }

        if let Some(content_length) = response.content_length {
            output.push_str(&format!("Content-Length: {}\n", content_length));
        }

        output.push_str("\nResponse Headers:\n");
        for (key, value) in &response.headers {
            output.push_str(&format!("  {}: {}\n", key, value));
        }

        output.push_str("\nResponse Body:\n");

        // Pretty print JSON if possible
        if let Some(content_type) = &response.content_type {
            if content_type.contains("application/json") {
                match serde_json::from_str::<serde_json::Value>(&response.body) {
                    Ok(json) => {
                        output.push_str(&serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone()));
                    }
                    Err(_) => {
                        output.push_str(&response.body);
                    }
                }
            } else {
                output.push_str(&response.body);
            }
        } else {
            output.push_str(&response.body);
        }

        // Build result with metadata
        let mut result = ToolResult::success(&call.id, self.name(), output)
            .with_metadata("status", serde_json::Value::Number(response.status.into()))
            .with_metadata("response_time_ms", serde_json::Value::Number(response.response_time.into()))
            .with_metadata("url", serde_json::Value::String(url));

        if let Some(content_type) = response.content_type {
            result = result.with_metadata("content_type", serde_json::Value::String(content_type));
        }

        if let Some(content_length) = response.content_length {
            result = result.with_metadata("content_length", serde_json::Value::Number(content_length.into()));
        }

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Validate URL parameter
        let _url = call.get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;

        // Validate method if provided
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
        Some(300) // 5 minutes max for HTTP requests
    }

    fn supports_parallel_execution(&self) -> bool {
        true // HTTP requests can run in parallel
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
        let schema = tool.schema();

        assert_eq!(schema.name, "http_client");
        assert!(!schema.description.is_empty());
        // The parameters field is a JSON value with the schema structure
        assert!(schema.parameters.is_object());
    }

    #[test]
    fn test_url_validation_allows_valid_urls() {
        assert!(validate_url_security("https://example.com/api").is_ok());
        assert!(validate_url_security("http://api.github.com/users").is_ok());
        assert!(validate_url_security("https://httpbin.org/get").is_ok());
    }

    #[test]
    fn test_url_validation_blocks_localhost() {
        assert!(validate_url_security("http://localhost/api").is_err());
        assert!(validate_url_security("http://127.0.0.1/api").is_err());
        assert!(validate_url_security("http://localhost:8080/api").is_err());
    }

    #[test]
    fn test_url_validation_blocks_internal_hostnames() {
        assert!(validate_url_security("http://server.local/api").is_err());
        assert!(validate_url_security("http://db.internal/api").is_err());
        assert!(validate_url_security("http://service.localhost/api").is_err());
    }

    #[test]
    fn test_url_validation_blocks_metadata_endpoints() {
        assert!(validate_url_security("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(validate_url_security("http://metadata.google.internal/computeMetadata/").is_err());
    }

    #[test]
    fn test_url_validation_blocks_non_http_schemes() {
        assert!(validate_url_security("file:///etc/passwd").is_err());
        assert!(validate_url_security("ftp://example.com/file").is_err());
        assert!(validate_url_security("gopher://example.com/").is_err());
    }

    #[test]
    fn test_url_validation_blocks_private_ips() {
        assert!(validate_url_security("http://10.0.0.1/api").is_err());
        assert!(validate_url_security("http://192.168.1.1/api").is_err());
        assert!(validate_url_security("http://172.16.0.1/api").is_err());
    }

    #[test]
    fn test_is_private_ip() {
        use std::net::Ipv4Addr;

        // Loopback
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

        // Private ranges
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));

        // Link-local
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));

        // Public IPs should return false
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }
}