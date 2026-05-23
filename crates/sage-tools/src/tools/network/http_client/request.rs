//! HTTP request building and execution

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Response, StatusCode, Url};
use tokio::time::timeout;
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

fn should_rewrite_redirect_to_get(status: StatusCode, method: &reqwest::Method) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER
    ) && *method != reqwest::Method::GET
        && *method != reqwest::Method::HEAD
}

async fn send_request_once(
    client: &reqwest::Client,
    params: &HttpClientParams,
    method: reqwest::Method,
    url: Url,
    timeout_secs: u64,
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

    timeout(Duration::from_secs(timeout_secs), request.send())
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
    let mut method = to_reqwest_method(&params.method);
    let follow_redirects = params.follow_redirects.unwrap_or(true);
    let mut current_url = Url::parse(&params.url).context("Invalid URL format")?;
    let mut include_body = true;
    let mut include_sensitive_headers = true;
    let mut redirect_count = 0;

    let start_time = std::time::Instant::now();

    let response = loop {
        let response = send_request_once(
            client,
            &params,
            method.clone(),
            current_url.clone(),
            timeout_secs,
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
        include_sensitive_headers = same_origin(response.url(), &next_url);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn test_graphql_request_creation() {
        let query = "query { user { name } }";
        let variables = json!({ "id": 1 });

        let request = create_graphql_request(query, Some(&variables));

        assert_eq!(request["query"], query);
        assert_eq!(request["variables"], variables);
    }

    #[test]
    fn test_to_reqwest_method() {
        assert_eq!(to_reqwest_method(&HttpMethod::Get), reqwest::Method::GET);
        assert_eq!(to_reqwest_method(&HttpMethod::Post), reqwest::Method::POST);
        assert_eq!(to_reqwest_method(&HttpMethod::Put), reqwest::Method::PUT);
        assert_eq!(
            to_reqwest_method(&HttpMethod::Delete),
            reqwest::Method::DELETE
        );
    }

    #[tokio::test]
    async fn test_http_request_validation_blocks_ipv4_mapped_loopback() {
        let result = validate_url_security("http://[::ffff:127.0.0.1]/").await;

        assert!(
            result.is_err(),
            "HTTP client request validation must reject IPv4-mapped loopback literals"
        );
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: String) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: tests that mutate proxy environment are serialized.
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: tests that mutate proxy environment are serialized.
            unsafe {
                match &self.previous {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    async fn spawn_redirecting_server() -> Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind redirect test server")?;
        let addr = listener
            .local_addr()
            .context("read redirect test server addr")?;
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

    #[tokio::test]
    #[serial]
    async fn test_http_request_rejects_redirect_to_loopback() -> Result<()> {
        let proxy_url = spawn_redirecting_server().await?;
        let _http_proxy = EnvGuard::set("HTTP_PROXY", proxy_url.clone());
        let _http_proxy_lower = EnvGuard::set("http_proxy", proxy_url);
        let _no_proxy = EnvGuard::set("NO_PROXY", String::new());
        let _no_proxy_lower = EnvGuard::set("no_proxy", String::new());

        let client = create_client(true, true, 5)?;
        let params = HttpClientParams {
            method: HttpMethod::Get,
            url: "http://1.1.1.1/".to_string(),
            headers: None,
            body: None,
            auth: None,
            timeout: Some(5),
            follow_redirects: Some(true),
            verify_ssl: Some(true),
            save_to_file: None,
            graphql_query: None,
            graphql_variables: None,
        };

        let result = execute_request(&client, params).await;

        assert!(
            result.is_err(),
            "HTTP request must reject redirects to loopback/private targets"
        );

        Ok(())
    }
}
