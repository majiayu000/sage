//! HTTP request building and execution

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Response, StatusCode, Url};
use tokio::time::{Instant, timeout};
use tracing::{debug, info};

use super::types::{AuthType, HttpClientParams, HttpMethod, HttpResponse, RequestBody};
use super::validate_url_security;
use crate::tools::network::redirect::{
    MAX_REDIRECTS, is_redirect_status, same_origin, validate_redirect_target,
};

/// Build request with authentication
pub fn add_auth(request: reqwest::RequestBuilder, auth: &AuthType) -> reqwest::RequestBuilder {
    match auth {
        AuthType::Bearer { token } => request.header("Authorization", format!("Bearer {}", token)),
        AuthType::Basic { username, password } => request.basic_auth(username, Some(password)),
        AuthType::ApiKey { key, value } => request.header(key, value),
    }
}

/// Add request body
pub fn add_body(
    request: reqwest::RequestBuilder,
    body: &RequestBody,
) -> Result<reqwest::RequestBuilder> {
    match body {
        RequestBody::Json(json) => Ok(request.json(json)),
        RequestBody::Text(text) => Ok(request.body(text.clone())),
        RequestBody::Form(form) => Ok(request.form(form)),
        RequestBody::Binary(data) => {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .context("Failed to decode base64 binary data")?;
            Ok(request.body(bytes))
        }
    }
}

/// Create GraphQL request body
pub fn create_graphql_request(
    query: &str,
    variables: Option<&serde_json::Value>,
) -> serde_json::Value {
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
pub fn create_client(
    verify_ssl: bool,
    _follow_redirects: bool,
    timeout_secs: u64,
) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_ssl)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("Sage-Agent-HTTP-Client/1.0")
        .build()
        .context("Failed to create HTTP client")
}

fn is_sensitive_header(header_name: &str) -> bool {
    matches!(
        header_name.to_ascii_lowercase().as_str(),
        "authorization" | "cookie" | "proxy-authorization" | "x-api-key"
    )
}

pub(super) fn should_rewrite_redirect_to_get(status: StatusCode, method: &reqwest::Method) -> bool {
    match status {
        StatusCode::SEE_OTHER => {
            *method != reqwest::Method::GET && *method != reqwest::Method::HEAD
        }
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND => *method == reqwest::Method::POST,
        _ => false,
    }
}

async fn send_request_once(
    client: &reqwest::Client,
    params: &HttpClientParams,
    method: reqwest::Method,
    url: Url,
    request_timeout: Duration,
    include_body: bool,
    include_sensitive_headers: bool,
) -> Result<Response> {
    debug!("Making HTTP request: {} {}", method, url);

    let mut request = client.request(method, url);

    if let Some(headers) = &params.headers {
        for (key, value) in headers {
            if include_sensitive_headers || !is_sensitive_header(key) {
                request = request.header(key, value);
            }
        }
    }

    if include_sensitive_headers {
        if let Some(auth) = &params.auth {
            request = add_auth(request, auth);
        }
    }

    if include_body {
        if let Some(query) = &params.graphql_query {
            let graphql_body = create_graphql_request(query, params.graphql_variables.as_ref());
            request = request.json(&graphql_body);
        } else if let Some(body) = &params.body {
            request = add_body(request, body)?;
        }
    }

    timeout(request_timeout, request.send())
        .await
        .context("Request timeout")?
        .context("HTTP request failed")
}

/// Execute HTTP request with full configuration
pub async fn execute_request(
    client: &reqwest::Client,
    params: HttpClientParams,
) -> Result<HttpResponse> {
    validate_url_security(&params.url).await?;

    let timeout_secs = params.timeout.unwrap_or(30);
    let request_timeout = Duration::from_secs(timeout_secs);
    let deadline = Instant::now()
        .checked_add(request_timeout)
        .ok_or_else(|| anyhow::anyhow!("timeout is too large"))?;
    let mut method = to_reqwest_method(&params.method);
    let follow_redirects = params.follow_redirects.unwrap_or(true);
    let mut current_url = Url::parse(&params.url).context("Invalid URL format")?;
    let mut include_body = true;
    let mut include_sensitive_headers = true;
    let mut redirect_count = 0;

    let start_time = std::time::Instant::now();

    let response = loop {
        let now = Instant::now();
        if now >= deadline {
            anyhow::bail!("Request timeout");
        }
        let remaining_timeout = deadline.saturating_duration_since(now);
        let response = send_request_once(
            client,
            &params,
            method.clone(),
            current_url.clone(),
            remaining_timeout,
            include_body,
            include_sensitive_headers,
        )
        .await?;

        if !follow_redirects || !is_redirect_status(response.status()) {
            break response;
        }

        if redirect_count >= MAX_REDIRECTS {
            anyhow::bail!("Redirect limit exceeded ({MAX_REDIRECTS})");
        }

        let next_url = validate_redirect_target(response.url(), response.headers()).await?;
        include_sensitive_headers =
            include_sensitive_headers && same_origin(response.url(), &next_url);
        if should_rewrite_redirect_to_get(response.status(), &method) {
            method = reqwest::Method::GET;
            include_body = false;
        }

        current_url = next_url;
        redirect_count += 1;
    };

    let response_time = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);
    let status = response.status().as_u16();

    let mut headers = HashMap::new();
    for (key, value) in response.headers() {
        if let Ok(value_str) = value.to_str() {
            headers.insert(key.to_string(), value_str.to_string());
        }
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let content_length = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());

    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if let Some(file_path) = &params.save_to_file {
        tokio::fs::write(file_path, &body)
            .await
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
                output.push_str(
                    &serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone()),
                );
                return output;
            }
        }
    }
    output.push_str(&response.body);
    output
}
