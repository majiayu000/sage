//! Shared redirect handling for network tools.

use anyhow::{Result, anyhow};
use reqwest::header::{HeaderMap, LOCATION};
use reqwest::{StatusCode, Url};

use super::validation::validate_url_security;

pub const MAX_REDIRECTS: usize = 10;

pub fn is_redirect_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

pub async fn validate_redirect_target(current_url: &Url, headers: &HeaderMap) -> Result<Url> {
    let location = headers
        .get(LOCATION)
        .ok_or_else(|| anyhow!("Redirect response missing Location header"))?
        .to_str()
        .map_err(|_| anyhow!("Redirect Location header is not valid UTF-8"))?;

    let next_url = current_url
        .join(location)
        .map_err(|error| anyhow!("Invalid redirect Location header: {}", error))?;

    validate_url_security(next_url.as_str())
        .await
        .map_err(|error| anyhow!("Redirect target failed URL validation: {}", error))?;

    Ok(next_url)
}

pub fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[tokio::test]
    async fn test_redirect_target_rejects_loopback() -> Result<()> {
        let current_url = Url::parse("http://1.1.1.1/start")?;
        let mut headers = HeaderMap::new();
        headers.insert(
            LOCATION,
            HeaderValue::from_static("http://127.0.0.1/private"),
        );

        let result = validate_redirect_target(&current_url, &headers).await;

        assert!(
            result.is_err(),
            "redirect validation must reject loopback targets"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_redirect_target_resolves_relative_location() -> Result<()> {
        let current_url = Url::parse("http://1.1.1.1/start/page")?;
        let mut headers = HeaderMap::new();
        headers.insert(LOCATION, HeaderValue::from_static("../next"));

        let next_url = validate_redirect_target(&current_url, &headers).await?;

        assert_eq!(next_url.as_str(), "http://1.1.1.1/next");

        Ok(())
    }
}
