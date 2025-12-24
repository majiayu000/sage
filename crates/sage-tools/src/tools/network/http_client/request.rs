//! HTTP request building and execution

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;
use tracing::{debug, info};

use super::types::{AuthType, HttpClientParams, HttpMethod, HttpResponse, RequestBody};
use super::validation::validate_url_security;

/// Build request with authentication
pub fn add_auth(request: reqwest::RequestBuilder, auth: &AuthType) -> reqwest::RequestBuilder {
    match auth {
        AuthType::Bearer { token } => {
            request.header("Authorization", format!("Bearer {}", token))
        }
        AuthType::Basic { username, password } => {
            request.basic_auth(username, Some(password))
        }
        AuthType::ApiKey { key, value } => {
            request.header(key, value)
        }
    }
}

/// Add request body
pub fn add_body(request: reqwest::RequestBuilder, body: &RequestBody) -> Result<reqwest::RequestBuilder> {
    match body {
        RequestBody::Json(json) => {
            Ok(request.json(json))
        }
        RequestBody::Text(text) => {
            Ok(request.body(text.clone()))
        }
        RequestBody::Form(form) => {
            Ok(request.form(form))
        }
        RequestBody::Binary(data) => {
            let bytes = base64::decode(data)
                .context("Failed to decode base64 binary data")?;
            Ok(request.body(bytes))
        }
    }
}

/// Create GraphQL request body
pub fn create_graphql_request(query: &str, variables: Option<&serde_json::Value>) -> serde_json::Value {
    let mut graphql_body = serde_json::json!({
        "query": query
    });

    if let Some(vars) = variables {
        graphql_body["variables"] = vars.clone();
    }

    graphql_body
}

/// Convert HTTP method to reqwest method
pub fn to_reqwest_method(method: &HttpMethod) -> reqwest::Method {
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

/// Create HTTP client with configuration
pub fn create_client(verify_ssl: bool, follow_redirects: bool, timeout_secs: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_ssl)
        .redirect(if follow_redirects {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("Sage-Agent-HTTP-Client/1.0")
        .build()
        .context("Failed to create HTTP client")
}

/// Execute HTTP request with full configuration
pub async fn execute_request(client: &reqwest::Client, params: HttpClientParams) -> Result<HttpResponse> {
    validate_url_security(&params.url).await?;

    let timeout_secs = params.timeout.unwrap_or(30);
    let method = to_reqwest_method(&params.method);

    debug!("Making HTTP request: {} {}", method, params.url);

    let mut request = client.request(method, &params.url);

    if let Some(headers) = &params.headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    if let Some(auth) = &params.auth {
        request = add_auth(request, auth);
    }

    if let Some(query) = &params.graphql_query {
        let graphql_body = create_graphql_request(query, params.graphql_variables.as_ref());
        request = request.json(&graphql_body);
    } else if let Some(body) = &params.body {
        request = add_body(request, body)?;
    }

    let start_time = std::time::Instant::now();

    let response = timeout(Duration::from_secs(timeout_secs), request.send())
        .await
        .context("Request timeout")?
        .context("HTTP request failed")?;

    let response_time = start_time.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

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

/// Format response for display
pub fn format_response(response: &HttpResponse) -> String {
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

    if let Some(content_type) = &response.content_type {
        if content_type.contains("application/json") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
                output.push_str(&serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone()));
                return output;
            }
        }
    }
    output.push_str(&response.body);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_graphql_request_creation() {
        let query = "query { user { name } }";
        let variables = Some(json!({ "id": 1 }));

        let request = create_graphql_request(query, variables.as_ref());

        assert_eq!(request["query"], query);
        assert_eq!(request["variables"], variables.unwrap());
    }

    #[test]
    fn test_to_reqwest_method() {
        assert_eq!(to_reqwest_method(&HttpMethod::Get), reqwest::Method::GET);
        assert_eq!(to_reqwest_method(&HttpMethod::Post), reqwest::Method::POST);
        assert_eq!(to_reqwest_method(&HttpMethod::Put), reqwest::Method::PUT);
        assert_eq!(to_reqwest_method(&HttpMethod::Delete), reqwest::Method::DELETE);
    }
}
