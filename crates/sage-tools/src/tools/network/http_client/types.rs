//! HTTP client type definitions

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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
