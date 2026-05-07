//! URL and input validation for network tools
//!
//! Provides security validation to prevent SSRF attacks and other
//! network-based vulnerabilities.

use std::net::IpAddr;

use anyhow::{Result, anyhow};
use url::Host;

/// Validate URL to prevent SSRF attacks
///
/// This function checks for:
/// - Valid URL format
/// - Only http/https schemes allowed
/// - Literal IP hosts (decimal, hex, IPv4-in-IPv6, etc.) routed
///   directly through `is_private_ip` so encoding tricks cannot
///   bypass the textual short-circuit.
/// - No localhost or internal hostnames
/// - No private IP addresses (after DNS resolution)
/// - No cloud metadata endpoints
pub async fn validate_url_security(url_str: &str) -> Result<()> {
    let url = url::Url::parse(url_str).map_err(|e| anyhow!("Invalid URL format: {}", e))?;

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

    let host = url.host().ok_or_else(|| anyhow!("URL must have a host"))?;

    // If the host is a literal IP address (in any encoding the URL
    // parser canonicalised — decimal `2130706433`, octal `0177.0.0.1`,
    // IPv4-mapped `[::ffff:127.0.0.1]`, etc.), route it through
    // `is_private_ip` directly. This closes the previous bypass where
    // a decimal-encoded loopback evaded the literal-string short-circuit.
    match host {
        Host::Ipv4(ip) => {
            if is_private_ip(&IpAddr::V4(ip)) {
                return Err(anyhow!(
                    "Requests to private/internal IP addresses are not allowed (literal {})",
                    ip
                ));
            }
            // Public IP literal: allow.
            return Ok(());
        }
        Host::Ipv6(ip) => {
            if is_private_ip(&IpAddr::V6(ip)) {
                return Err(anyhow!(
                    "Requests to private/internal IP addresses are not allowed (literal {})",
                    ip
                ));
            }
            return Ok(());
        }
        Host::Domain(_) => {}
    }

    // Domain-name path. host_str() is safe here because Host::Domain
    // round-trips through the same string.
    let host_str = url
        .host_str()
        .ok_or_else(|| anyhow!("URL must have a host"))?;

    // Block localhost variants
    let host_lower = host_str.to_lowercase();
    if host_lower == "localhost" {
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
            host_str
        ));
    }

    // Block known cloud metadata hostnames before DNS resolution so a
    // poisoned resolver cannot return a public IP for them.
    if host_lower == "metadata.google.internal" {
        return Err(anyhow!(
            "Requests to cloud metadata endpoints are not allowed"
        ));
    }

    // Try to resolve the hostname asynchronously and check if it's a private IP
    if let Ok(addrs) = tokio::net::lookup_host(format!("{}:80", host_str)).await {
        for addr in addrs {
            if is_private_ip(&addr.ip()) {
                return Err(anyhow!(
                    "Requests to private/internal IP addresses are not allowed (resolved to {})",
                    addr.ip()
                ));
            }
        }
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
            if ipv6.is_loopback() {
                return true;
            }
            // fe80::/10 - Link-local
            if (ipv6.segments()[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            // fc00::/7 - Unique local
            if (ipv6.segments()[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // ::ffff:0:0/96 — IPv4-mapped IPv6. Recurse on the embedded
            // v4 address so e.g. ::ffff:127.0.0.1 is treated as private.
            if let Some(v4) = ipv6.to_ipv4_mapped() {
                return is_private_ip(&IpAddr::V4(v4));
            }
            // ::ffff:a.b.c.d (legacy IPv4-compatible, deprecated but
            // still parseable). `to_ipv4()` covers both forms.
            if let Some(v4) = ipv6.to_ipv4() {
                // Skip the all-zeros / unspecified case which `to_ipv4`
                // returns as 0.0.0.0; not a real address.
                if !v4.is_unspecified() {
                    return is_private_ip(&IpAddr::V4(v4));
                }
            }
            false
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

        // Public IPs should return false
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn test_is_private_ip_ipv4_mapped_ipv6() {
        // ::ffff:127.0.0.1 must be detected as private (loopback),
        // closing the IPv4-in-IPv6 encoding bypass.
        let mapped: std::net::Ipv6Addr = "::ffff:127.0.0.1".parse().unwrap();
        assert!(is_private_ip(&IpAddr::V6(mapped)));

        // ::ffff:10.0.0.1 also private.
        let mapped_priv: std::net::Ipv6Addr = "::ffff:10.0.0.1".parse().unwrap();
        assert!(is_private_ip(&IpAddr::V6(mapped_priv)));

        // ::ffff:8.8.8.8 — public IPv4 mapped into IPv6 — must NOT be
        // flagged as private.
        let mapped_public: std::net::Ipv6Addr = "::ffff:8.8.8.8".parse().unwrap();
        assert!(!is_private_ip(&IpAddr::V6(mapped_public)));
    }

    #[tokio::test]
    async fn test_url_validation_blocks_decimal_encoded_loopback() {
        // `http://2130706433/` is the dotted-quad 127.0.0.1 in decimal.
        // The url crate canonicalizes it into Host::Ipv4, and the new
        // literal-IP path routes it through `is_private_ip` regardless
        // of the textual form.
        let result = validate_url_security("http://2130706433/").await;
        assert!(
            result.is_err(),
            "decimal-encoded loopback must be rejected: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_ipv4_mapped_ipv6_loopback() {
        // `http://[::ffff:127.0.0.1]/` is the IPv4-mapped IPv6 form of
        // the loopback. Must be rejected via the IPv4-mapped recursion
        // in `is_private_ip`.
        let result = validate_url_security("http://[::ffff:127.0.0.1]/").await;
        assert!(
            result.is_err(),
            "IPv4-mapped IPv6 loopback must be rejected: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_ipv6_loopback_literal() {
        // `http://[::1]/` is the bracketed IPv6 loopback literal. The
        // url crate parses this as Host::Ipv6(::1) and the literal-IP
        // branch routes it through is_private_ip.
        let result = validate_url_security("http://[::1]/").await;
        assert!(
            result.is_err(),
            "IPv6 loopback literal must be rejected: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_link_local_ipv6() {
        // fe80::/10 — IPv6 link-local. Caught by the existing
        // is_private_ip rule plus the new literal-IP branch.
        let result = validate_url_security("http://[fe80::1]/").await;
        assert!(
            result.is_err(),
            "IPv6 link-local must be rejected: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_url_validation_blocks_metadata_ipv4_literal() {
        // 169.254.169.254 (EC2/GCP metadata) is now caught by the
        // literal-IP branch routing through is_private_ip's
        // 169.254.0.0/16 link-local rule. The hostname-based check for
        // `metadata.google.internal` still runs for the domain path.
        let result = validate_url_security("http://169.254.169.254/").await;
        assert!(
            result.is_err(),
            "metadata IPv4 literal must be rejected: {result:?}"
        );
    }
}
