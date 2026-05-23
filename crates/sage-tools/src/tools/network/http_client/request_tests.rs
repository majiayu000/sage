use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::Instant;

use super::request::{
    create_graphql_request, execute_request, should_rewrite_redirect_to_get, to_reqwest_method,
};
use super::{HttpClientParams, HttpMethod, validate_url_security};

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

#[test]
fn test_redirect_method_rewrite_preserves_non_post_for_301_302() {
    assert!(should_rewrite_redirect_to_get(
        StatusCode::SEE_OTHER,
        &reqwest::Method::PUT
    ));
    assert!(should_rewrite_redirect_to_get(
        StatusCode::FOUND,
        &reqwest::Method::POST
    ));
    assert!(should_rewrite_redirect_to_get(
        StatusCode::MOVED_PERMANENTLY,
        &reqwest::Method::POST
    ));
    assert!(!should_rewrite_redirect_to_get(
        StatusCode::FOUND,
        &reqwest::Method::PUT
    ));
    assert!(!should_rewrite_redirect_to_get(
        StatusCode::MOVED_PERMANENTLY,
        &reqwest::Method::PATCH
    ));
    assert!(!should_rewrite_redirect_to_get(
        StatusCode::TEMPORARY_REDIRECT,
        &reqwest::Method::POST
    ));
    assert!(!should_rewrite_redirect_to_get(
        StatusCode::PERMANENT_REDIRECT,
        &reqwest::Method::DELETE
    ));
}

#[tokio::test]
async fn test_http_request_validation_blocks_ipv4_mapped_loopback() {
    let result = validate_url_security("http://[::ffff:127.0.0.1]/").await;

    assert!(
        result.is_err(),
        "HTTP client request validation must reject IPv4-mapped loopback literals"
    );
}

async fn spawn_http_client_loopback_redirect_proxy() -> Result<String> {
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

async fn spawn_sensitive_header_capture_proxy() -> Result<(String, oneshot::Receiver<String>)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind header capture test proxy")?;
    let addr = listener.local_addr().context("read header capture addr")?;
    let proxy_url = format!("http://{}", addr);
    let (request_sender, request_receiver) = oneshot::channel();

    tokio::spawn(async move {
        let mut request_sender = Some(request_sender);

        for request_index in 0..3 {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };

            let mut buffer = [0_u8; 4096];
            let Ok(bytes_read) = socket.read(&mut buffer).await else {
                return;
            };
            let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).into_owned();

            let response = match request_index {
                0 => {
                    "HTTP/1.1 302 Found\r\nLocation: http://2.2.2.2/step\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                }
                1 => {
                    "HTTP/1.1 302 Found\r\nLocation: http://2.2.2.2/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                }
                _ => {
                    let Some(sender) = request_sender.take() else {
                        return;
                    };
                    if sender.send(request_text).is_err() {
                        return;
                    }
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_string()
                }
            };

            if socket.write_all(response.as_bytes()).await.is_err() {
                return;
            }
        }
    });

    Ok((proxy_url, request_receiver))
}

async fn spawn_slow_redirect_proxy() -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind slow redirect test proxy")?;
    let addr = listener
        .local_addr()
        .context("read slow redirect test proxy addr")?;
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
async fn test_http_request_rejects_redirect_to_loopback() -> Result<()> {
    let proxy_url = spawn_http_client_loopback_redirect_proxy().await?;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(proxy_url)?)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
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

#[tokio::test]
async fn test_cross_origin_redirect_keeps_sensitive_headers_stripped() -> Result<()> {
    let (proxy_url, final_request_receiver) = spawn_sensitive_header_capture_proxy().await?;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(proxy_url)?)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer secret".to_string());
    headers.insert("Cookie".to_string(), "session=secret".to_string());
    headers.insert("X-Api-Key".to_string(), "secret".to_string());

    let params = HttpClientParams {
        method: HttpMethod::Get,
        url: "http://1.1.1.1/".to_string(),
        headers: Some(headers),
        body: None,
        auth: None,
        timeout: Some(5),
        follow_redirects: Some(true),
        verify_ssl: Some(true),
        save_to_file: None,
        graphql_query: None,
        graphql_variables: None,
    };

    let response = execute_request(&client, params).await?;
    let final_request = final_request_receiver
        .await
        .context("capture final redirected request")?;

    assert_eq!(response.status, 200);
    assert!(
        !final_request
            .to_ascii_lowercase()
            .contains("authorization:"),
        "Authorization must stay stripped after any cross-origin redirect"
    );
    assert!(
        !final_request.to_ascii_lowercase().contains("cookie:"),
        "Cookie must stay stripped after any cross-origin redirect"
    );
    assert!(
        !final_request.to_ascii_lowercase().contains("x-api-key:"),
        "X-Api-Key must stay stripped after any cross-origin redirect"
    );

    Ok(())
}

#[tokio::test]
async fn test_redirect_chain_uses_single_timeout_budget() -> Result<()> {
    let proxy_url = spawn_slow_redirect_proxy().await?;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(proxy_url)?)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let params = HttpClientParams {
        method: HttpMethod::Get,
        url: "http://1.1.1.1/".to_string(),
        headers: None,
        body: None,
        auth: None,
        timeout: Some(1),
        follow_redirects: Some(true),
        verify_ssl: Some(true),
        save_to_file: None,
        graphql_query: None,
        graphql_variables: None,
    };

    let start = Instant::now();
    let result = execute_request(&client, params).await;

    assert!(
        result.is_err(),
        "redirect chain must use one timeout budget"
    );
    assert!(
        start.elapsed() < Duration::from_millis(1300),
        "redirect chain exceeded the single timeout budget"
    );

    Ok(())
}
