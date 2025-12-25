//! Validation rules for input validation

use super::schema::{FieldSchema, FieldType};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

/// Validation rule types
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Minimum string length
    MinLength(usize),

    /// Maximum string length
    MaxLength(usize),

    /// Minimum numeric value
    MinValue(f64),

    /// Maximum numeric value
    MaxValue(f64),

    /// Regex pattern match
    Pattern(String),

    /// Email format
    Email,

    /// URL format
    Url,

    /// File path format
    Path,

    /// Non-empty check (string, array, or object)
    NonEmpty,

    /// Minimum array length
    MinItems(usize),

    /// Maximum array length
    MaxItems(usize),

    /// Unique array items
    UniqueItems,

    /// Custom validation with name and predicate
    Custom { name: String, message: String },
}

impl ValidationRule {
    /// Validate a value against this rule
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        match self {
            ValidationRule::MinLength(min) => {
                if let Some(s) = value.as_str() {
                    if s.len() < *min {
                        return Err(format!(
                            "String length {} is less than minimum {}",
                            s.len(),
                            min
                        ));
                    }
                }
                Ok(())
            }

            ValidationRule::MaxLength(max) => {
                if let Some(s) = value.as_str() {
                    if s.len() > *max {
                        return Err(format!("String length {} exceeds maximum {}", s.len(), max));
                    }
                }
                Ok(())
            }

            ValidationRule::MinValue(min) => {
                if let Some(n) = value.as_f64() {
                    if n < *min {
                        return Err(format!("Value {} is less than minimum {}", n, min));
                    }
                }
                Ok(())
            }

            ValidationRule::MaxValue(max) => {
                if let Some(n) = value.as_f64() {
                    if n > *max {
                        return Err(format!("Value {} exceeds maximum {}", n, max));
                    }
                }
                Ok(())
            }

            ValidationRule::Pattern(pattern) => {
                if let Some(s) = value.as_str() {
                    match Regex::new(pattern) {
                        Ok(re) => {
                            if !re.is_match(s) {
                                return Err(format!(
                                    "Value '{}' does not match pattern '{}'",
                                    s, pattern
                                ));
                            }
                        }
                        Err(e) => {
                            return Err(format!("Invalid regex pattern: {}", e));
                        }
                    }
                }
                Ok(())
            }

            ValidationRule::Email => {
                if let Some(s) = value.as_str() {
                    // Basic email validation
                    static EMAIL_RE: OnceLock<Regex> = OnceLock::new();
                    let email_re = EMAIL_RE.get_or_init(|| {
                        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
                    });
                    if !email_re.is_match(s) {
                        return Err(format!("'{}' is not a valid email address", s));
                    }
                }
                Ok(())
            }

            ValidationRule::Url => {
                if let Some(s) = value.as_str() {
                    // Basic URL validation
                    static URL_RE: OnceLock<Regex> = OnceLock::new();
                    let url_re = URL_RE.get_or_init(|| {
                        Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
                    });
                    if !url_re.is_match(s) {
                        return Err(format!("'{}' is not a valid URL", s));
                    }
                }
                Ok(())
            }

            ValidationRule::Path => {
                if let Some(s) = value.as_str() {
                    // Check for path-like string (not empty, no null bytes)
                    if s.is_empty() {
                        return Err("Path cannot be empty".to_string());
                    }
                    if s.contains('\0') {
                        return Err("Path cannot contain null bytes".to_string());
                    }
                    // Check for common path traversal attempts
                    if s.contains("..") && (s.contains("../") || s.contains("..\\")) {
                        return Err("Path contains potential traversal".to_string());
                    }
                }
                Ok(())
            }

            ValidationRule::NonEmpty => match value {
                Value::String(s) if s.is_empty() => Err("String cannot be empty".to_string()),
                Value::Array(arr) if arr.is_empty() => Err("Array cannot be empty".to_string()),
                Value::Object(obj) if obj.is_empty() => Err("Object cannot be empty".to_string()),
                _ => Ok(()),
            },

            ValidationRule::MinItems(min) => {
                if let Some(arr) = value.as_array() {
                    if arr.len() < *min {
                        return Err(format!("Array has {} items, minimum is {}", arr.len(), min));
                    }
                }
                Ok(())
            }

            ValidationRule::MaxItems(max) => {
                if let Some(arr) = value.as_array() {
                    if arr.len() > *max {
                        return Err(format!("Array has {} items, maximum is {}", arr.len(), max));
                    }
                }
                Ok(())
            }

            ValidationRule::UniqueItems => {
                if let Some(arr) = value.as_array() {
                    let mut seen = std::collections::HashSet::new();
                    for item in arr {
                        let key = item.to_string();
                        if seen.contains(&key) {
                            return Err("Array contains duplicate items".to_string());
                        }
                        seen.insert(key);
                    }
                }
                Ok(())
            }

            ValidationRule::Custom { name, message } => {
                // Custom rules need external validation
                // This is a placeholder that always passes
                // Actual validation should be done by the validator
                let _ = (name, message);
                Ok(())
            }
        }
    }

    /// Get rule name for error messages
    pub fn rule_name(&self) -> &str {
        match self {
            ValidationRule::MinLength(_) => "min_length",
            ValidationRule::MaxLength(_) => "max_length",
            ValidationRule::MinValue(_) => "min_value",
            ValidationRule::MaxValue(_) => "max_value",
            ValidationRule::Pattern(_) => "pattern",
            ValidationRule::Email => "email",
            ValidationRule::Url => "url",
            ValidationRule::Path => "path",
            ValidationRule::NonEmpty => "non_empty",
            ValidationRule::MinItems(_) => "min_items",
            ValidationRule::MaxItems(_) => "max_items",
            ValidationRule::UniqueItems => "unique_items",
            ValidationRule::Custom { name, .. } => name,
        }
    }
}

/// Collection of common validation rules
pub struct RuleSet;

impl RuleSet {
    /// Rules for a non-empty string
    pub fn non_empty_string() -> Vec<ValidationRule> {
        vec![ValidationRule::NonEmpty]
    }

    /// Rules for a string with length constraints
    pub fn string_length(min: usize, max: usize) -> Vec<ValidationRule> {
        vec![
            ValidationRule::MinLength(min),
            ValidationRule::MaxLength(max),
        ]
    }

    /// Rules for a number in range
    pub fn number_range(min: f64, max: f64) -> Vec<ValidationRule> {
        vec![ValidationRule::MinValue(min), ValidationRule::MaxValue(max)]
    }

    /// Rules for a positive number
    pub fn positive_number() -> Vec<ValidationRule> {
        vec![ValidationRule::MinValue(0.0)]
    }

    /// Rules for a valid email
    pub fn email() -> Vec<ValidationRule> {
        vec![ValidationRule::Email]
    }

    /// Rules for a valid URL
    pub fn url() -> Vec<ValidationRule> {
        vec![ValidationRule::Url]
    }

    /// Rules for a safe file path
    pub fn safe_path() -> Vec<ValidationRule> {
        vec![ValidationRule::Path, ValidationRule::NonEmpty]
    }
}

/// Common predefined rules
pub struct CommonRules;

impl CommonRules {
    /// File path validation
    pub fn file_path() -> FieldSchema {
        FieldSchema::new(FieldType::String)
            .required(true)
            .non_empty()
            .path()
    }

    /// Command validation (non-empty string, max length)
    pub fn command() -> FieldSchema {
        FieldSchema::new(FieldType::String)
            .required(true)
            .non_empty()
            .max_length(10000)
    }

    /// Positive integer
    pub fn positive_integer() -> FieldSchema {
        FieldSchema::new(FieldType::Integer)
            .required(true)
            .min_value(1.0)
    }

    /// Non-negative integer
    pub fn non_negative_integer() -> FieldSchema {
        FieldSchema::new(FieldType::Integer)
            .required(true)
            .min_value(0.0)
    }

    /// Optional boolean with default
    pub fn optional_boolean(default: bool) -> FieldSchema {
        FieldSchema::new(FieldType::Boolean)
            .required(false)
            .default_value(serde_json::Value::Bool(default))
    }

    /// Timeout in seconds (1-3600)
    pub fn timeout_seconds() -> FieldSchema {
        FieldSchema::new(FieldType::Integer)
            .required(false)
            .min_value(1.0)
            .max_value(3600.0)
            .default_value(serde_json::json!(120))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_length() {
        let rule = ValidationRule::MinLength(5);
        assert!(rule.validate(&serde_json::json!("hello")).is_ok());
        assert!(rule.validate(&serde_json::json!("hi")).is_err());
    }

    #[test]
    fn test_max_length() {
        let rule = ValidationRule::MaxLength(5);
        assert!(rule.validate(&serde_json::json!("hi")).is_ok());
        assert!(rule.validate(&serde_json::json!("hello world")).is_err());
    }

    #[test]
    fn test_min_value() {
        let rule = ValidationRule::MinValue(0.0);
        assert!(rule.validate(&serde_json::json!(5)).is_ok());
        assert!(rule.validate(&serde_json::json!(-1)).is_err());
    }

    #[test]
    fn test_max_value() {
        let rule = ValidationRule::MaxValue(100.0);
        assert!(rule.validate(&serde_json::json!(50)).is_ok());
        assert!(rule.validate(&serde_json::json!(150)).is_err());
    }

    #[test]
    fn test_pattern() {
        let rule = ValidationRule::Pattern(r"^\d{3}-\d{4}$".to_string());
        assert!(rule.validate(&serde_json::json!("123-4567")).is_ok());
        assert!(rule.validate(&serde_json::json!("12-34567")).is_err());
    }

    #[test]
    fn test_email() {
        let rule = ValidationRule::Email;
        assert!(
            rule.validate(&serde_json::json!("test@example.com"))
                .is_ok()
        );
        assert!(rule.validate(&serde_json::json!("invalid-email")).is_err());
    }

    #[test]
    fn test_url() {
        let rule = ValidationRule::Url;
        assert!(
            rule.validate(&serde_json::json!("https://example.com"))
                .is_ok()
        );
        assert!(rule.validate(&serde_json::json!("not-a-url")).is_err());
    }

    #[test]
    fn test_path() {
        let rule = ValidationRule::Path;
        assert!(
            rule.validate(&serde_json::json!("/home/user/file.txt"))
                .is_ok()
        );
        assert!(rule.validate(&serde_json::json!("")).is_err());
    }

    #[test]
    fn test_non_empty() {
        let rule = ValidationRule::NonEmpty;
        assert!(rule.validate(&serde_json::json!("text")).is_ok());
        assert!(rule.validate(&serde_json::json!("")).is_err());
        assert!(rule.validate(&serde_json::json!([])).is_err());
        assert!(rule.validate(&serde_json::json!({})).is_err());
    }

    #[test]
    fn test_min_items() {
        let rule = ValidationRule::MinItems(2);
        assert!(rule.validate(&serde_json::json!([1, 2, 3])).is_ok());
        assert!(rule.validate(&serde_json::json!([1])).is_err());
    }

    #[test]
    fn test_max_items() {
        let rule = ValidationRule::MaxItems(3);
        assert!(rule.validate(&serde_json::json!([1, 2])).is_ok());
        assert!(rule.validate(&serde_json::json!([1, 2, 3, 4])).is_err());
    }

    #[test]
    fn test_unique_items() {
        let rule = ValidationRule::UniqueItems;
        assert!(rule.validate(&serde_json::json!([1, 2, 3])).is_ok());
        assert!(rule.validate(&serde_json::json!([1, 2, 2])).is_err());
    }

    #[test]
    fn test_rule_set() {
        let rules = RuleSet::string_length(1, 10);
        assert_eq!(rules.len(), 2);

        let rules = RuleSet::number_range(0.0, 100.0);
        assert_eq!(rules.len(), 2);
    }
}
