//! Feedback bundle redaction helpers.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const REDACTED: &str = "[REDACTED]";
const REDACTED_PATH: &str = "[REDACTED_PATH]";

static BEARER_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._\-+/=]{8,}").expect("valid bearer token regex")
});

static KEY_VALUE_SECRET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)["']?\b([A-Z0-9_-]*(?:api[_-]?key|access[_-]?token|refresh[_-]?token|token|secret|password|cookie|credential|private[_-]?key)[A-Z0-9_-]*)\b["']?\s*[:=]\s*(?:"[^"\r\n]*"|'[^'\r\n]*'|[^"',\s}\]]+)"#,
    )
    .expect("valid secret regex")
});

static AUTHORIZATION_SECRET_RE: Lazy<Regex> = Lazy::new(|| {
    match Regex::new(
        r#"(?i)["']?\b(authorization|x-api-key)\b["']?\s*[:=]\s*(?:"[^"\r\n]*"|'[^'\r\n]*'|(?:bearer|basic|token|api-key|apikey)\s+[^"',\s=}\]]+(?:\s+[^"',\s=}\]]+)?|[^"',\s}\]]+)"#,
    ) {
        Ok(regex) => regex,
        Err(error) => panic!("invalid authorization secret regex: {error}"),
    }
});

static PROVIDER_KEY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(sk-[A-Za-z0-9._\-]{8,}|gh[pousr]_[A-Za-z0-9_]{12,}|AIza[A-Za-z0-9_\-]{12,})\b")
        .expect("valid provider key regex")
});

static SENSITIVE_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)
        (/(Users|home)/[^\s"',}]+)
        |([A-Za-z]:\\Users\\[^\s"',}]+)
        |([^\s"',}]*\.(env|pem|key|p12|pfx)\b)
        |([^\s"',}]*\.(ssh|aws|kube|docker)[^\s"',}]*)
    "#,
    )
    .expect("valid sensitive path regex")
});

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReport {
    pub replacements: u64,
    pub redacted_keys: Vec<String>,
    pub uncertain_fields: Vec<String>,
}

impl RedactionReport {
    fn record_replacements(&mut self, before: &str, after: &str) {
        if before != after {
            self.replacements = self.replacements.saturating_add(1);
        }
    }

    pub fn merge(&mut self, other: RedactionReport) {
        self.replacements = self.replacements.saturating_add(other.replacements);
        self.redacted_keys.extend(other.redacted_keys);
        self.uncertain_fields.extend(other.uncertain_fields);
        self.redacted_keys.sort();
        self.redacted_keys.dedup();
        self.uncertain_fields.sort();
        self.uncertain_fields.dedup();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedText {
    pub value: String,
    pub report: RedactionReport,
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticRedactor;

impl DiagnosticRedactor {
    pub fn new() -> Self {
        Self
    }

    pub fn redact_text(&self, input: &str) -> RedactedText {
        let mut report = RedactionReport::default();
        let mut value = input.to_string();

        for replacement in [
            (&*AUTHORIZATION_SECRET_RE, "$1=[REDACTED]"),
            (&*BEARER_TOKEN_RE, "Bearer [REDACTED]"),
            (&*KEY_VALUE_SECRET_RE, "$1=[REDACTED]"),
            (&*PROVIDER_KEY_RE, REDACTED),
            (&*SENSITIVE_PATH_RE, REDACTED_PATH),
        ] {
            let before = value;
            value = replacement
                .0
                .replace_all(&before, replacement.1)
                .into_owned();
            report.record_replacements(&before, &value);
        }

        RedactedText { value, report }
    }

    pub fn redact_json_value(&self, mut value: Value) -> (Value, RedactionReport) {
        let mut report = RedactionReport::default();
        self.redact_json_in_place(&mut value, &mut report, None);
        (value, report)
    }

    fn redact_json_in_place(
        &self,
        value: &mut Value,
        report: &mut RedactionReport,
        current_key: Option<&str>,
    ) {
        match value {
            Value::Object(map) => {
                for (key, child) in map.iter_mut() {
                    if is_sensitive_key(key) {
                        *child = Value::String(REDACTED.to_string());
                        report.replacements = report.replacements.saturating_add(1);
                        report.redacted_keys.push(key.clone());
                    } else if should_redact_unknown(current_key, key) {
                        *child = Value::String(REDACTED.to_string());
                        report.replacements = report.replacements.saturating_add(1);
                        report.uncertain_fields.push(key.clone());
                    } else {
                        self.redact_json_in_place(child, report, Some(key));
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.redact_json_in_place(item, report, current_key);
                }
            }
            Value::String(text) => {
                let redacted = self.redact_text(text);
                report.merge(redacted.report);
                *text = redacted.value;
            }
            _ => {}
        }
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
        || normalized.contains("credential")
        || normalized.contains("private_key")
}

fn should_redact_unknown(parent_key: Option<&str>, key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    matches!(parent_key, Some("payload") | Some("context"))
        && (normalized.contains("raw") || normalized.contains("unknown"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_redaction_redacts_tokens_cookies_provider_keys_and_paths() {
        let redactor = DiagnosticRedactor::new();
        let input = "Authorization: Bearer sk-secret-token-value cookie=sessionid=abc /Users/alice/project/.env OPENAI_API_KEY=sk-abc123456789 ANTHROPIC_API_KEY=plain-secret AWS_SECRET_ACCESS_KEY=aws-secret DATABASE_PASSWORD=db-secret";

        let redacted = redactor.redact_text(input);

        assert!(!redacted.value.contains("sk-secret-token-value"));
        assert!(!redacted.value.contains("abc123456789"));
        assert!(!redacted.value.contains("plain-secret"));
        assert!(!redacted.value.contains("aws-secret"));
        assert!(!redacted.value.contains("db-secret"));
        assert!(!redacted.value.contains("/Users/alice"));
        assert!(redacted.value.contains("[REDACTED]"));
        assert!(redacted.value.contains("[REDACTED_PATH]"));
        assert!(redacted.report.replacements >= 3);
    }

    #[test]
    fn diagnostics_redaction_redacts_quoted_and_space_separated_secret_values() {
        let redactor = DiagnosticRedactor::new();
        let input = "DATABASE_PASSWORD=\"foo bar,baz\" Authorization: Basic abc def token='x y,z'";

        let redacted = redactor.redact_text(input);

        assert!(!redacted.value.contains("foo"));
        assert!(!redacted.value.contains("bar"));
        assert!(!redacted.value.contains("baz"));
        assert!(!redacted.value.contains("abc def"));
        assert!(!redacted.value.contains("x y"));
        assert!(redacted.value.contains("DATABASE_PASSWORD=[REDACTED]"));
        assert!(redacted.value.contains("Authorization=[REDACTED]"));
        assert!(redacted.value.contains("token=[REDACTED]"));
    }

    #[test]
    fn diagnostics_redaction_redacts_json_shaped_quoted_secret_keys() {
        let redactor = DiagnosticRedactor::new();
        let input = r#"{"DATABASE_PASSWORD":"foo bar","authorization":"Basic abc def"}"#;

        let redacted = redactor.redact_text(input);

        assert!(!redacted.value.contains("foo bar"));
        assert!(!redacted.value.contains("abc def"));
        assert!(redacted.value.contains("DATABASE_PASSWORD=[REDACTED]"));
        assert!(redacted.value.contains("authorization=[REDACTED]"));
    }

    #[test]
    fn diagnostics_redaction_strictly_redacts_sensitive_json_fields() {
        let redactor = DiagnosticRedactor::new();
        let value = serde_json::json!({
            "provider": "openai",
            "api_key": "sk-secret",
            "cookie": "sid=abc",
            "context": {
                "raw_payload": "possibly sensitive"
            }
        });

        let (redacted, report) = redactor.redact_json_value(value);

        let text = serde_json::to_string(&redacted).unwrap();
        assert!(!text.contains("sk-secret"));
        assert!(!text.contains("sid=abc"));
        assert!(!text.contains("possibly sensitive"));
        assert!(report.redacted_keys.contains(&"api_key".to_string()));
        assert!(report.uncertain_fields.contains(&"raw_payload".to_string()));
    }
}
