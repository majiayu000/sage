//! LLM provider error sanitization helpers.

use crate::error::SageError;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

const MAX_ERROR_TEXT_CHARS: usize = 1_024;
const REDACTED: &str = "[REDACTED]";

static BEARER_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._\-+/=]{8,}").expect("valid bearer token regex")
});

static KEY_VALUE_SECRET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)\b(api[_-]?key|access[_-]?token|refresh[_-]?token|token|secret|password|authorization|x-api-key)\b\s*[:=]\s*["']?[^"',\s}]+"#,
    )
    .expect("valid key/value secret regex")
});

/// Sanitize provider error text by redacting secrets and truncating large payloads.
pub fn sanitize_provider_error_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty error response body>".to_string();
    }

    if let Ok(mut json) = serde_json::from_str::<Value>(trimmed) {
        redact_json_value(&mut json);
        let serialized =
            serde_json::to_string(&json).unwrap_or_else(|_| "<unserializable error>".to_string());
        return truncate_with_suffix(serialized);
    }

    let redacted = redact_inline_secrets(trimmed);
    truncate_with_suffix(redacted)
}

fn redact_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_key(key) {
                    *val = Value::String(REDACTED.to_string());
                } else {
                    redact_json_value(val);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                redact_json_value(item);
            }
        }
        Value::String(s) => {
            *s = redact_inline_secrets(s);
        }
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
    normalized.contains("api_key")
        || normalized.contains("access_token")
        || normalized.contains("refresh_token")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("authorization")
        || normalized.contains("x_api_key")
        || normalized.contains("cookie")
        || normalized.contains("private_key")
}

fn redact_inline_secrets(input: &str) -> String {
    let redacted_bearer = BEARER_TOKEN_RE.replace_all(input, "Bearer [REDACTED]");
    KEY_VALUE_SECRET_RE
        .replace_all(&redacted_bearer, "$1=[REDACTED]")
        .into_owned()
}

fn truncate_with_suffix(input: String) -> String {
    let char_count = input.chars().count();
    if char_count <= MAX_ERROR_TEXT_CHARS {
        return input;
    }

    let truncated: String = input.chars().take(MAX_ERROR_TEXT_CHARS).collect();
    format!(
        "{}... [truncated {} chars]",
        truncated,
        char_count - MAX_ERROR_TEXT_CHARS
    )
}

/// Build a SageError from a non-success HTTP response.
pub async fn handle_http_error(response: reqwest::Response, provider: &str) -> SageError {
    let status = response.status();
    let error_text = response.text().await.unwrap_or_default();
    let sanitized = sanitize_provider_error_text(&error_text);
    SageError::llm(format!(
        "{} API error (status {}): {}",
        provider, status, sanitized
    ))
}

/// Build a SageError from a non-success HTTP response (streaming variant, no status).
pub async fn handle_stream_http_error(response: reqwest::Response, provider: &str) -> SageError {
    let error_text = response.text().await.unwrap_or_default();
    let sanitized = sanitize_provider_error_text(&error_text);
    SageError::llm(format!("{} streaming API error: {}", provider, sanitized))
}

/// Build a SageError from a JSON parse failure.
pub fn handle_parse_error(err: reqwest::Error, provider: &str) -> SageError {
    SageError::llm_with_context(
        format!("Failed to parse {} response: {}", provider, err),
        format!(
            "Failed to deserialize {} API response as JSON",
            provider
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::sanitize_provider_error_text;

    #[test]
    fn redacts_json_sensitive_fields() {
        let raw = r#"{"error":{"message":"bad request","api_key":"sk-secret","token":"abc123"}}"#;
        let sanitized = sanitize_provider_error_text(raw);
        assert!(!sanitized.contains("sk-secret"));
        assert!(!sanitized.contains("abc123"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_bearer_token_in_plain_text() {
        let raw = "Authorization: Bearer sk-very-secret-token-value";
        let sanitized = sanitize_provider_error_text(raw);
        assert!(!sanitized.contains("sk-very-secret-token-value"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
