//! Pattern matching for hooks
//!
//! Provides flexible pattern matching functionality for hook triggers,
//! supporting exact matches, wildcards, pipe-separated alternatives, and regex.

use regex::Regex;

/// Check if a pattern contains regex metacharacters
fn contains_regex_metacharacters(pattern: &str) -> bool {
    pattern
        .chars()
        .any(|c| matches!(c, '^' | '$' | '.' | '*' | '+' | '?' | '[' | ']' | '(' | ')' | '{' | '}' | '\\'))
}

/// Match a value against a pattern
///
/// # Pattern Types
///
/// - `None` or `Some("*")`: Matches everything (wildcard)
/// - Pipe-separated (`"a|b|c"`): Matches if value equals or contains any segment
/// - Regex pattern (contains `^$.*+?[](){}\\`): Uses regex matching
/// - Simple string: Exact match only
///
/// # Examples
///
/// ```
/// use sage_core::hooks::matcher::matches;
///
/// // Wildcard matches everything
/// assert!(matches(None, "anything"));
/// assert!(matches(Some("*"), "anything"));
///
/// // Exact match (simple strings without regex metacharacters)
/// assert!(matches(Some("bash"), "bash"));
/// assert!(!matches(Some("bash"), "python"));
/// assert!(!matches(Some("bash"), "bash_script")); // No substring match
///
/// // Pipe-separated alternatives
/// assert!(matches(Some("bash|python|node"), "bash"));
/// assert!(matches(Some("bash|python|node"), "python"));
/// assert!(!matches(Some("bash|python"), "ruby"));
///
/// // Contains match for pipe-separated
/// assert!(matches(Some("test|demo"), "this is a test"));
///
/// // Regex patterns (contain metacharacters)
/// assert!(matches(Some("^test.*"), "testing123"));
/// assert!(matches(Some(r"^\d+$"), "12345"));
/// assert!(matches(Some("test.file"), "test_file")); // . is regex metachar
/// ```
pub fn matches(pattern: Option<&str>, value: &str) -> bool {
    match pattern {
        // No pattern or wildcard = match all
        None => true,
        Some("*") => true,

        // Pipe-separated alternatives
        Some(p) if p.contains('|') => {
            p.split('|')
                .map(|s| s.trim())
                .any(|segment| value == segment || value.contains(segment))
        }

        // Pattern with regex metacharacters - use regex matching
        Some(p) if contains_regex_metacharacters(p) => {
            // Exact match always works
            if value == p {
                return true;
            }
            // Try regex match
            Regex::new(p)
                .map(|re| re.is_match(value))
                .unwrap_or(false)
        }

        // Simple pattern without metacharacters - exact match only
        Some(p) => value == p,
    }
}

/// A pattern matcher with a name for debugging
#[derive(Debug, Clone)]
pub struct PatternMatcher {
    pattern: Option<String>,
    name: String,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new(name: impl Into<String>, pattern: Option<String>) -> Self {
        Self {
            pattern,
            name: name.into(),
        }
    }

    /// Check if this matcher matches the given value
    pub fn matches(&self, value: &str) -> bool {
        matches(self.pattern.as_deref(), value)
    }

    /// Get the pattern
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Get the name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if this is a wildcard matcher
    pub fn is_wildcard(&self) -> bool {
        self.pattern.is_none() || self.pattern.as_deref() == Some("*")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_none() {
        assert!(matches(None, "anything"));
        assert!(matches(None, ""));
        assert!(matches(None, "bash"));
    }

    #[test]
    fn test_wildcard_star() {
        assert!(matches(Some("*"), "anything"));
        assert!(matches(Some("*"), ""));
        assert!(matches(Some("*"), "bash"));
    }

    #[test]
    fn test_exact_match() {
        assert!(matches(Some("bash"), "bash"));
        assert!(matches(Some("python"), "python"));
        assert!(!matches(Some("bash"), "python"));
        assert!(!matches(Some("bash"), "bash_script"));
    }

    #[test]
    fn test_pipe_alternatives_exact() {
        let pattern = Some("bash|python|node");

        assert!(matches(pattern, "bash"));
        assert!(matches(pattern, "python"));
        assert!(matches(pattern, "node"));
        assert!(!matches(pattern, "ruby"));
        assert!(!matches(pattern, "go"));
    }

    #[test]
    fn test_pipe_alternatives_contains() {
        let pattern = Some("test|demo");

        assert!(matches(pattern, "test"));
        assert!(matches(pattern, "demo"));
        assert!(matches(pattern, "this is a test"));
        assert!(matches(pattern, "demo_file"));
        assert!(!matches(pattern, "production"));
    }

    #[test]
    fn test_pipe_alternatives_with_spaces() {
        let pattern = Some("bash | python | node");

        assert!(matches(pattern, "bash"));
        assert!(matches(pattern, "python"));
        assert!(matches(pattern, "node"));
    }

    #[test]
    fn test_regex_simple() {
        assert!(matches(Some("^test"), "testing"));
        assert!(matches(Some("^test"), "test123"));
        assert!(!matches(Some("^test"), "my_test"));

        assert!(matches(Some("test$"), "my_test"));
        assert!(!matches(Some("test$"), "testing"));
    }

    #[test]
    fn test_regex_digit_pattern() {
        assert!(matches(Some(r"^\d+$"), "123"));
        assert!(matches(Some(r"^\d+$"), "456789"));
        assert!(!matches(Some(r"^\d+$"), "abc"));
        assert!(!matches(Some(r"^\d+$"), "12abc"));
    }

    #[test]
    fn test_regex_word_pattern() {
        assert!(matches(Some(r"^\w+_test$"), "unit_test"));
        assert!(matches(Some(r"^\w+_test$"), "integration_test"));
        assert!(!matches(Some(r"^\w+_test$"), "test_unit"));
        assert!(!matches(Some(r"^\w+_test$"), "unit_test_case"));
    }

    #[test]
    fn test_regex_any_pattern() {
        assert!(matches(Some(".*tool.*"), "my_tool_name"));
        assert!(matches(Some(".*tool.*"), "tool"));
        assert!(matches(Some(".*tool.*"), "toolbox"));
        assert!(!matches(Some(".*tool.*"), "my_helper"));
    }

    #[test]
    fn test_regex_invalid_pattern() {
        // Invalid regex should not match (and not panic)
        assert!(!matches(Some("[invalid"), "test"));
        assert!(!matches(Some("(unclosed"), "test"));
    }

    #[test]
    fn test_pattern_matcher_new() {
        let matcher = PatternMatcher::new("test_matcher", Some("bash".to_string()));
        assert_eq!(matcher.name(), "test_matcher");
        assert_eq!(matcher.pattern(), Some("bash"));
    }

    #[test]
    fn test_pattern_matcher_matches() {
        let matcher = PatternMatcher::new("test", Some("bash|python".to_string()));
        assert!(matcher.matches("bash"));
        assert!(matcher.matches("python"));
        assert!(!matcher.matches("ruby"));
    }

    #[test]
    fn test_pattern_matcher_wildcard() {
        let matcher1 = PatternMatcher::new("wildcard1", None);
        assert!(matcher1.is_wildcard());
        assert!(matcher1.matches("anything"));

        let matcher2 = PatternMatcher::new("wildcard2", Some("*".to_string()));
        assert!(matcher2.is_wildcard());
        assert!(matcher2.matches("anything"));

        let matcher3 = PatternMatcher::new("specific", Some("bash".to_string()));
        assert!(!matcher3.is_wildcard());
    }

    #[test]
    fn test_empty_value() {
        assert!(matches(None, ""));
        assert!(matches(Some("*"), ""));
        assert!(matches(Some(""), ""));
        assert!(!matches(Some("bash"), ""));
    }

    #[test]
    fn test_empty_pattern() {
        assert!(matches(Some(""), ""));
        assert!(!matches(Some(""), "bash"));
    }

    #[test]
    fn test_complex_regex() {
        // Test email-like pattern
        let email_pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$");
        assert!(matches(email_pattern, "test@example.com"));
        assert!(matches(email_pattern, "user.name+tag@example.co.uk"));
        assert!(!matches(email_pattern, "invalid.email"));
        assert!(!matches(email_pattern, "@example.com"));

        // Test path-like pattern
        let path_pattern = Some(r"^/[a-z]+/[a-z]+$");
        assert!(matches(path_pattern, "/usr/bin"));
        assert!(matches(path_pattern, "/home/user"));
        assert!(!matches(path_pattern, "/usr/bin/bash"));
        assert!(!matches(path_pattern, "usr/bin"));
    }

    #[test]
    fn test_case_sensitive_exact() {
        assert!(matches(Some("Bash"), "Bash"));
        assert!(!matches(Some("Bash"), "bash"));
        assert!(!matches(Some("bash"), "Bash"));
    }

    #[test]
    fn test_case_sensitive_regex() {
        // Case-sensitive regex
        assert!(matches(Some("^Test"), "Test"));
        assert!(!matches(Some("^Test"), "test"));

        // Case-insensitive regex
        assert!(matches(Some("(?i)^test"), "Test"));
        assert!(matches(Some("(?i)^test"), "TEST"));
        assert!(matches(Some("(?i)^test"), "test"));
    }

    #[test]
    fn test_special_characters() {
        // Dot in pattern (should work as regex)
        assert!(matches(Some("test.file"), "test_file"));
        assert!(matches(Some("test.file"), "testXfile"));

        // Escaped dot for exact match
        assert!(matches(Some(r"test\.file"), "test.file"));
        assert!(!matches(Some(r"test\.file"), "testXfile"));
    }

    #[test]
    fn test_multiline_not_supported() {
        // Newlines in value - pattern matching should still work
        assert!(matches(Some("test"), "test"));
        assert!(!matches(Some("^line1$"), "line1\nline2"));

        // Patterns don't cross newlines by default
        assert!(matches(Some("(?s)line1.*line2"), "line1\nline2")); // (?s) enables dot-all mode
    }

    #[test]
    fn test_unicode_support() {
        assert!(matches(Some("你好"), "你好"));
        assert!(matches(Some(".*emoji.*"), "test 😀 emoji"));
        assert!(matches(Some("café"), "café"));
    }

    #[test]
    fn test_pattern_matcher_debug() {
        let matcher = PatternMatcher::new("debug_test", Some("pattern".to_string()));
        let debug_str = format!("{:?}", matcher);
        assert!(debug_str.contains("pattern"));
        assert!(debug_str.contains("debug_test"));
    }
}
