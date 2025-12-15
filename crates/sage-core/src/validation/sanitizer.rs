//! Input sanitization for security

use serde_json::{Map, Value};

/// Sanitization options
#[derive(Debug, Clone)]
pub struct SanitizeOptions {
    /// Remove null values
    pub remove_nulls: bool,

    /// Trim string whitespace
    pub trim_strings: bool,

    /// Maximum string length (truncate if exceeded)
    pub max_string_length: Option<usize>,

    /// Maximum array length (truncate if exceeded)
    pub max_array_length: Option<usize>,

    /// Maximum object depth
    pub max_depth: Option<usize>,

    /// Remove potentially dangerous HTML/script content
    pub remove_html: bool,

    /// Remove control characters from strings
    pub remove_control_chars: bool,

    /// Normalize unicode
    pub normalize_unicode: bool,

    /// Remove empty strings
    pub remove_empty_strings: bool,

    /// Remove keys with dangerous names
    pub remove_dangerous_keys: bool,
}

impl Default for SanitizeOptions {
    fn default() -> Self {
        Self {
            remove_nulls: false,
            trim_strings: true,
            max_string_length: Some(100_000),
            max_array_length: Some(10_000),
            max_depth: Some(50),
            remove_html: true,
            remove_control_chars: true,
            normalize_unicode: false,
            remove_empty_strings: false,
            remove_dangerous_keys: true,
        }
    }
}

impl SanitizeOptions {
    /// Create strict sanitization options
    pub fn strict() -> Self {
        Self {
            remove_nulls: true,
            trim_strings: true,
            max_string_length: Some(10_000),
            max_array_length: Some(1_000),
            max_depth: Some(10),
            remove_html: true,
            remove_control_chars: true,
            normalize_unicode: true,
            remove_empty_strings: true,
            remove_dangerous_keys: true,
        }
    }

    /// Create permissive sanitization options
    pub fn permissive() -> Self {
        Self {
            remove_nulls: false,
            trim_strings: false,
            max_string_length: None,
            max_array_length: None,
            max_depth: None,
            remove_html: false,
            remove_control_chars: false,
            normalize_unicode: false,
            remove_empty_strings: false,
            remove_dangerous_keys: false,
        }
    }
}

/// Input sanitizer
pub struct InputSanitizer {
    options: SanitizeOptions,
}

impl InputSanitizer {
    /// Create new sanitizer with options
    pub fn new(options: SanitizeOptions) -> Self {
        Self { options }
    }

    /// Sanitize a JSON value
    pub fn sanitize(&self, value: &Value) -> Value {
        self.sanitize_with_depth(value, 0)
    }

    /// Sanitize with depth tracking
    fn sanitize_with_depth(&self, value: &Value, depth: usize) -> Value {
        // Check max depth
        if let Some(max_depth) = self.options.max_depth {
            if depth > max_depth {
                return Value::Null;
            }
        }

        match value {
            Value::Null => {
                if self.options.remove_nulls {
                    // Return a marker that will be filtered out
                    Value::Null
                } else {
                    Value::Null
                }
            }

            Value::Bool(b) => Value::Bool(*b),

            Value::Number(n) => Value::Number(n.clone()),

            Value::String(s) => self.sanitize_string(s),

            Value::Array(arr) => self.sanitize_array(arr, depth),

            Value::Object(obj) => self.sanitize_object(obj, depth),
        }
    }

    /// Sanitize a string
    fn sanitize_string(&self, s: &str) -> Value {
        let mut result = s.to_string();

        // Trim whitespace
        if self.options.trim_strings {
            result = result.trim().to_string();
        }

        // Remove control characters
        if self.options.remove_control_chars {
            result = result
                .chars()
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
                .collect();
        }

        // Remove HTML/script tags
        if self.options.remove_html {
            result = self.remove_html_tags(&result);
        }

        // Truncate if too long
        if let Some(max_len) = self.options.max_string_length {
            if result.len() > max_len {
                result = result.chars().take(max_len).collect();
            }
        }

        // Check for empty string
        if self.options.remove_empty_strings && result.is_empty() {
            return Value::Null;
        }

        Value::String(result)
    }

    /// Remove HTML tags from string
    fn remove_html_tags(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut in_tag = false;

        for c in s.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(c),
                _ => {}
            }
        }

        // Also remove common dangerous patterns
        let result = result
            .replace("javascript:", "")
            .replace("vbscript:", "")
            .replace("data:", "")
            .replace("onclick", "")
            .replace("onerror", "")
            .replace("onload", "");

        result
    }

    /// Sanitize an array
    fn sanitize_array(&self, arr: &[Value], depth: usize) -> Value {
        let mut result: Vec<Value> = arr
            .iter()
            .map(|v| self.sanitize_with_depth(v, depth + 1))
            .collect();

        // Remove nulls if configured
        if self.options.remove_nulls {
            result.retain(|v| !v.is_null());
        }

        // Truncate if too long
        if let Some(max_len) = self.options.max_array_length {
            result.truncate(max_len);
        }

        Value::Array(result)
    }

    /// Sanitize an object
    fn sanitize_object(&self, obj: &Map<String, Value>, depth: usize) -> Value {
        let mut result = Map::new();

        for (key, value) in obj {
            // Skip dangerous keys
            if self.options.remove_dangerous_keys && self.is_dangerous_key(key) {
                continue;
            }

            let sanitized_value = self.sanitize_with_depth(value, depth + 1);

            // Skip null values if configured
            if self.options.remove_nulls && sanitized_value.is_null() {
                continue;
            }

            // Sanitize key as well
            let sanitized_key = if self.options.trim_strings {
                key.trim().to_string()
            } else {
                key.clone()
            };

            result.insert(sanitized_key, sanitized_value);
        }

        Value::Object(result)
    }

    /// Check if a key name is potentially dangerous
    fn is_dangerous_key(&self, key: &str) -> bool {
        let lower = key.to_lowercase();

        // Prototype pollution
        if lower == "__proto__" || lower == "constructor" || lower == "prototype" {
            return true;
        }

        // Server-side template injection
        if lower.contains("{{") || lower.contains("}}") {
            return true;
        }

        // Command injection patterns
        if lower.contains(";") || lower.contains("|") || lower.contains("$") {
            return true;
        }

        false
    }
}

/// Convenience function for quick sanitization
pub fn sanitize_json(value: &Value) -> Value {
    InputSanitizer::new(SanitizeOptions::default()).sanitize(value)
}

/// Convenience function for strict sanitization
pub fn sanitize_json_strict(value: &Value) -> Value {
    InputSanitizer::new(SanitizeOptions::strict()).sanitize(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_strings() {
        let sanitizer = InputSanitizer::new(SanitizeOptions::default());
        let input = serde_json::json!("  hello world  ");
        let result = sanitizer.sanitize(&input);
        assert_eq!(result, serde_json::json!("hello world"));
    }

    #[test]
    fn test_remove_html() {
        let sanitizer = InputSanitizer::new(SanitizeOptions::default());
        let input = serde_json::json!("<script>alert('xss')</script>Hello");
        let result = sanitizer.sanitize(&input);
        assert_eq!(result, serde_json::json!("alert('xss')Hello"));
    }

    #[test]
    fn test_remove_control_chars() {
        let sanitizer = InputSanitizer::new(SanitizeOptions::default());
        let input = serde_json::json!("hello\x00world\x1f");
        let result = sanitizer.sanitize(&input);
        assert_eq!(result, serde_json::json!("helloworld"));
    }

    #[test]
    fn test_max_string_length() {
        let options = SanitizeOptions {
            max_string_length: Some(5),
            ..Default::default()
        };
        let sanitizer = InputSanitizer::new(options);
        let input = serde_json::json!("hello world");
        let result = sanitizer.sanitize(&input);
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[test]
    fn test_max_array_length() {
        let options = SanitizeOptions {
            max_array_length: Some(3),
            ..Default::default()
        };
        let sanitizer = InputSanitizer::new(options);
        let input = serde_json::json!([1, 2, 3, 4, 5]);
        let result = sanitizer.sanitize(&input);
        assert_eq!(result, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_remove_nulls() {
        let options = SanitizeOptions {
            remove_nulls: true,
            ..Default::default()
        };
        let sanitizer = InputSanitizer::new(options);
        let input = serde_json::json!({
            "name": "Alice",
            "age": null,
            "items": [1, null, 3]
        });
        let result = sanitizer.sanitize(&input);
        let obj = result.as_object().unwrap();
        assert!(!obj.contains_key("age"));
        assert_eq!(obj["items"], serde_json::json!([1, 3]));
    }

    #[test]
    fn test_dangerous_keys() {
        let sanitizer = InputSanitizer::new(SanitizeOptions::default());
        let input = serde_json::json!({
            "__proto__": "polluted",
            "constructor": "polluted",
            "name": "Alice"
        });
        let result = sanitizer.sanitize(&input);
        let obj = result.as_object().unwrap();
        assert!(!obj.contains_key("__proto__"));
        assert!(!obj.contains_key("constructor"));
        assert!(obj.contains_key("name"));
    }

    #[test]
    fn test_max_depth() {
        let options = SanitizeOptions {
            max_depth: Some(2),
            ..Default::default()
        };
        let sanitizer = InputSanitizer::new(options);
        let input = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "deep"
                    }
                }
            }
        });
        let result = sanitizer.sanitize(&input);
        // At depth 3, the value should be null
        let level3 = &result["level1"]["level2"]["level3"];
        assert!(level3.is_null());
    }

    #[test]
    fn test_remove_empty_strings() {
        let options = SanitizeOptions {
            remove_empty_strings: true,
            ..Default::default()
        };
        let sanitizer = InputSanitizer::new(options);
        let input = serde_json::json!({
            "name": "Alice",
            "empty": "",
            "whitespace": "   "
        });
        let result = sanitizer.sanitize(&input);
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("name"));
        // Empty and whitespace-only strings become null (then removed)
        assert!(obj["empty"].is_null());
        assert!(obj["whitespace"].is_null());
    }

    #[test]
    fn test_nested_sanitization() {
        let sanitizer = InputSanitizer::new(SanitizeOptions::default());
        let input = serde_json::json!({
            "user": {
                "name": "  Alice  ",
                "bio": "<b>Hello</b>"
            },
            "items": ["  item1  ", "<script>bad</script>"]
        });
        let result = sanitizer.sanitize(&input);
        assert_eq!(result["user"]["name"], "Alice");
        assert_eq!(result["user"]["bio"], "Hello");
        assert_eq!(result["items"][0], "item1");
    }

    #[test]
    fn test_permissive_options() {
        let sanitizer = InputSanitizer::new(SanitizeOptions::permissive());
        let input = serde_json::json!({
            "__proto__": "kept",
            "html": "<script>alert(1)</script>",
            "whitespace": "  not trimmed  "
        });
        let result = sanitizer.sanitize(&input);
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("__proto__"));
        assert!(result["html"].as_str().unwrap().contains("<script>"));
        assert!(result["whitespace"].as_str().unwrap().starts_with("  "));
    }

    #[test]
    fn test_convenience_functions() {
        let input = serde_json::json!({
            "name": "  Alice  ",
            "__proto__": "pollution"
        });

        let result = sanitize_json(&input);
        assert_eq!(result["name"], "Alice");
        assert!(!result.as_object().unwrap().contains_key("__proto__"));
    }
}
