//! URL validation and security checks for HTTP client

use anyhow::{Result, anyhow};
use std::net::IpAddr;
use url::Url;

/// Validate URL to prevent SSRF attacks
pub async fn validate_url_security(url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).map_err(|e| anyhow!("Invalid URL format: {}", e))?;

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

    let host = url
        .host_str()
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

    // Try to resolve the hostname asynchronously and check if it's a private IP
    if let Ok(addrs) = tokio::net::lookup_host(format!("{}:80", host)).await {
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
pub fn is_private_ip(ip: &IpAddr) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_url_validation_allows_valid_urls() {
        assert!(
            validate_url_security("https://example.com/api")
                .await
                .is_ok()
        );
        assert!(
            validate_url_security("http://api.github.com/users")
                .await
                .is_ok()
        );
        assert!(
            validate_url_security("https://httpbin.org/get")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_localhost() {
        assert!(validate_url_security("http://localhost/api").await.is_err());
        assert!(validate_url_security("http://127.0.0.1/api").await.is_err());
        assert!(
            validate_url_security("http://localhost:8080/api")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_internal_hostnames() {
        assert!(
            validate_url_security("http://server.local/api")
                .await
                .is_err()
        );
        assert!(
            validate_url_security("http://db.internal/api")
                .await
                .is_err()
        );
        assert!(
            validate_url_security("http://service.localhost/api")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_metadata_endpoints() {
        assert!(
            validate_url_security("http://169.254.169.254/latest/meta-data/")
                .await
                .is_err()
        );
        assert!(
            validate_url_security("http://metadata.google.internal/computeMetadata/")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_non_http_schemes() {
        assert!(validate_url_security("file:///etc/passwd").await.is_err());
        assert!(
            validate_url_security("ftp://example.com/file")
                .await
                .is_err()
        );
        assert!(
            validate_url_security("gopher://example.com/")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_private_ips() {
        assert!(validate_url_security("http://10.0.0.1/api").await.is_err());
        assert!(
            validate_url_security("http://192.168.1.1/api")
                .await
                .is_err()
        );
        assert!(
            validate_url_security("http://172.16.0.1/api")
                .await
                .is_err()
        );
    }

    #[test]
    fn test_is_private_ip() {
        // Loopback
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

        // Private ranges
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));

        // Link-local
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));

        // Public IPs should not be private
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }
}
