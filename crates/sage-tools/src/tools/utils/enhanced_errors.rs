//! Enhanced error handling for Sage Tools

use sage_core::tools::base::ToolError;
use std::collections::HashMap;

/// Enhanced tool error with additional context and suggestions
#[derive(Debug)]
pub struct EnhancedToolError {
    /// Original tool error message
    pub original_error_message: String,
    /// Original tool error type
    pub original_error_type: String,
    /// Additional context information
    pub context: HashMap<String, String>,
    /// Suggested solutions
    pub suggestions: Vec<String>,
    /// Error category for better classification
    pub category: ErrorCategory,
    /// Whether this error is recoverable
    pub recoverable: bool,
}

/// Error categories for better classification
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    /// User input validation errors
    UserInput,
    /// File system related errors
    FileSystem,
    /// Network related errors
    Network,
    /// Permission and security errors
    Permission,
    /// Configuration errors
    Configuration,
    /// System resource errors (memory, disk, etc.)
    Resource,
    /// External dependency errors
    Dependency,
    /// Internal logic errors
    Internal,
}

impl EnhancedToolError {
    /// Create a new enhanced error from a tool error
    pub fn new(original_error: ToolError) -> Self {
        let original_error_message = original_error.to_string();
        let original_error_type = Self::get_error_type(&original_error);
        let category = Self::categorize_error(&original_error);
        let recoverable = Self::is_recoverable(&original_error);

        Self {
            original_error_message,
            original_error_type,
            context: HashMap::new(),
            suggestions: Vec::new(),
            category,
            recoverable,
        }
    }

    /// Get error type as string
    fn get_error_type(error: &ToolError) -> String {
        match error {
            ToolError::InvalidArguments(_) => "InvalidArguments".to_string(),
            ToolError::Io(_) => "Io".to_string(),
            ToolError::PermissionDenied(_) => "PermissionDenied".to_string(),
            ToolError::NotFound(_) => "NotFound".to_string(),
            ToolError::Timeout => "Timeout".to_string(),
            ToolError::Json(_) => "Json".to_string(),
            ToolError::ExecutionFailed(_) => "ExecutionFailed".to_string(),
            ToolError::ValidationFailed(_) => "ValidationFailed".to_string(),
            ToolError::Cancelled => "Cancelled".to_string(),
            ToolError::Other(_) => "Other".to_string(),
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Add multiple suggestions
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions.extend(suggestions);
        self
    }

    /// Set error category
    pub fn with_category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }

    /// Set recoverable flag
    pub fn with_recoverable(mut self, recoverable: bool) -> Self {
        self.recoverable = recoverable;
        self
    }

    /// Categorize error based on its type
    fn categorize_error(error: &ToolError) -> ErrorCategory {
        match error {
            ToolError::InvalidArguments(_) => ErrorCategory::UserInput,
            ToolError::Io(_) => ErrorCategory::FileSystem,
            ToolError::PermissionDenied(_) => ErrorCategory::Permission,
            ToolError::NotFound(_) => ErrorCategory::FileSystem,
            ToolError::Timeout => ErrorCategory::Resource,
            ToolError::Json(_) => ErrorCategory::UserInput,
            ToolError::ExecutionFailed(_) => ErrorCategory::Internal,
            ToolError::ValidationFailed(_) => ErrorCategory::UserInput,
            ToolError::Cancelled => ErrorCategory::Internal,
            ToolError::Other(_) => ErrorCategory::Internal,
        }
    }

    /// Determine if error is recoverable
    fn is_recoverable(error: &ToolError) -> bool {
        match error {
            ToolError::InvalidArguments(_) => true,
            ToolError::Io(_) => false,
            ToolError::PermissionDenied(_) => false,
            ToolError::NotFound(_) => true,
            ToolError::Timeout => true,
            ToolError::Json(_) => true,
            ToolError::ExecutionFailed(_) => false,
            ToolError::ValidationFailed(_) => true,
            ToolError::Cancelled => false,
            ToolError::Other(_) => false,
        }
    }

    /// Generate user-friendly error message
    pub fn user_friendly_message(&self) -> String {
        let mut message = String::new();

        // Add main error message
        message.push_str(&format!("❌ {}\n", self.original_error_message));

        // Add category information
        message.push_str(&format!("📂 Category: {:?}\n", self.category));

        // Add context if available
        if !self.context.is_empty() {
            message.push_str("\n📋 Context:\n");
            for (key, value) in &self.context {
                message.push_str(&format!("  • {}: {}\n", key, value));
            }
        }

        // Add suggestions if available
        if !self.suggestions.is_empty() {
            message.push_str("\n💡 Suggestions:\n");
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                message.push_str(&format!("  {}. {}\n", i + 1, suggestion));
            }
        }

        // Add recovery information
        if self.recoverable {
            message.push_str(
                "\n🔄 This error may be recoverable. Please try the suggestions above.\n",
            );
        } else {
            message.push_str("\n⚠️  This error requires manual intervention to resolve.\n");
        }

        message
    }

    /// Convert to JSON for structured logging
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": self.original_error_message,
            "error_type": self.original_error_type,
            "category": format!("{:?}", self.category),
            "recoverable": self.recoverable,
            "context": self.context,
            "suggestions": self.suggestions,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

impl std::fmt::Display for EnhancedToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.user_friendly_message())
    }
}

impl std::error::Error for EnhancedToolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None // We can't store the original error due to lifetime issues
    }
}

impl From<ToolError> for EnhancedToolError {
    fn from(error: ToolError) -> Self {
        Self::new(error)
    }
}

/// Helper functions for common error scenarios
pub mod helpers {
    use super::*;

    /// Create an enhanced file not found error
    pub fn file_not_found(file_path: &str) -> EnhancedToolError {
        EnhancedToolError::new(ToolError::NotFound(format!(
            "File not found: {}",
            file_path
        )))
        .with_context("file_path", file_path)
        .with_suggestion("Check if the file path is correct")
        .with_suggestion("Ensure the file exists in the specified location")
        .with_suggestion("Verify you have read permissions for the file")
        .with_category(ErrorCategory::FileSystem)
    }

    /// Create an enhanced permission denied error
    pub fn permission_denied(operation: &str, resource: &str) -> EnhancedToolError {
        EnhancedToolError::new(ToolError::PermissionDenied(format!(
            "Permission denied: cannot {} {}",
            operation, resource
        )))
        .with_context("operation", operation)
        .with_context("resource", resource)
        .with_suggestion("Check file/directory permissions")
        .with_suggestion("Run with appropriate user privileges")
        .with_suggestion("Contact system administrator if needed")
        .with_category(ErrorCategory::Permission)
    }

    /// Create an enhanced invalid argument error
    pub fn invalid_argument(parameter: &str, value: &str, expected: &str) -> EnhancedToolError {
        EnhancedToolError::new(ToolError::InvalidArguments(format!(
            "Invalid argument '{}': got '{}', expected {}",
            parameter, value, expected
        )))
        .with_context("parameter", parameter)
        .with_context("provided_value", value)
        .with_context("expected_format", expected)
        .with_suggestion(format!(
            "Provide a valid value for parameter '{}'",
            parameter
        ))
        .with_suggestion(format!("Expected format: {}", expected))
        .with_category(ErrorCategory::UserInput)
    }

    /// Create an enhanced timeout error
    pub fn timeout_error(operation: &str, timeout_seconds: u64) -> EnhancedToolError {
        EnhancedToolError::new(ToolError::Timeout)
            .with_context("operation", operation)
            .with_context("timeout_seconds", timeout_seconds.to_string())
            .with_suggestion("Try increasing the timeout value")
            .with_suggestion("Check if the operation is resource-intensive")
            .with_suggestion("Verify system resources are available")
            .with_category(ErrorCategory::Resource)
    }

    /// Create an enhanced configuration error
    pub fn configuration_error(config_key: &str, issue: &str) -> EnhancedToolError {
        EnhancedToolError::new(ToolError::Other(format!(
            "Configuration error for '{}': {}",
            config_key, issue
        )))
        .with_context("config_key", config_key)
        .with_context("issue", issue)
        .with_suggestion("Check the configuration file")
        .with_suggestion("Verify the configuration value format")
        .with_suggestion("Reset to default configuration if needed")
        .with_category(ErrorCategory::Configuration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_enhanced_tool_error_creation() {
        let tool_error = ToolError::InvalidArguments("Invalid input".to_string());
        let enhanced = EnhancedToolError::new(tool_error);

        assert_eq!(enhanced.original_error_type, "InvalidArguments");
        assert_eq!(enhanced.category, ErrorCategory::UserInput);
        assert!(enhanced.recoverable);
    }

    #[test]
    fn test_enhanced_tool_error_with_context() {
        let tool_error = ToolError::NotFound("File missing".to_string());
        let enhanced = EnhancedToolError::new(tool_error)
            .with_context("file_path", "/test/path")
            .with_context("operation", "read");

        assert_eq!(enhanced.context.len(), 2);
        assert_eq!(enhanced.context.get("file_path"), Some(&"/test/path".to_string()));
        assert_eq!(enhanced.context.get("operation"), Some(&"read".to_string()));
    }

    #[test]
    fn test_enhanced_tool_error_with_suggestions() {
        let tool_error = ToolError::ValidationFailed("Bad format".to_string());
        let enhanced = EnhancedToolError::new(tool_error)
            .with_suggestion("Check the input format")
            .with_suggestion("Refer to documentation");

        assert_eq!(enhanced.suggestions.len(), 2);
        assert_eq!(enhanced.suggestions[0], "Check the input format");
        assert_eq!(enhanced.suggestions[1], "Refer to documentation");
    }

    #[test]
    fn test_enhanced_tool_error_with_multiple_suggestions() {
        let tool_error = ToolError::Timeout;
        let suggestions = vec![
            "Increase timeout value".to_string(),
            "Check network connection".to_string(),
        ];
        let enhanced = EnhancedToolError::new(tool_error).with_suggestions(suggestions.clone());

        assert_eq!(enhanced.suggestions, suggestions);
    }

    #[test]
    fn test_enhanced_tool_error_with_category() {
        let tool_error = ToolError::Other("Network issue".to_string());
        let enhanced = EnhancedToolError::new(tool_error)
            .with_category(ErrorCategory::Network);

        assert_eq!(enhanced.category, ErrorCategory::Network);
    }

    #[test]
    fn test_enhanced_tool_error_with_recoverable() {
        let tool_error = ToolError::Timeout;
        let enhanced = EnhancedToolError::new(tool_error)
            .with_recoverable(false);

        assert!(!enhanced.recoverable);
    }

    #[test]
    fn test_error_category_user_input() {
        let errors = vec![
            ToolError::InvalidArguments("test".to_string()),
            ToolError::Json(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "test"))),
            ToolError::ValidationFailed("test".to_string()),
        ];

        for error in errors {
            let enhanced = EnhancedToolError::new(error);
            assert_eq!(enhanced.category, ErrorCategory::UserInput);
        }
    }

    #[test]
    fn test_error_category_file_system() {
        let errors = vec![
            ToolError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
            ToolError::NotFound("test".to_string()),
        ];

        for error in errors {
            let enhanced = EnhancedToolError::new(error);
            assert_eq!(enhanced.category, ErrorCategory::FileSystem);
        }
    }

    #[test]
    fn test_error_category_permission() {
        let error = ToolError::PermissionDenied("Access denied".to_string());
        let enhanced = EnhancedToolError::new(error);
        assert_eq!(enhanced.category, ErrorCategory::Permission);
    }

    #[test]
    fn test_error_category_resource() {
        let error = ToolError::Timeout;
        let enhanced = EnhancedToolError::new(error);
        assert_eq!(enhanced.category, ErrorCategory::Resource);
    }

    #[test]
    fn test_error_category_internal() {
        let errors = vec![
            ToolError::ExecutionFailed("test".to_string()),
            ToolError::Cancelled,
            ToolError::Other("test".to_string()),
        ];

        for error in errors {
            let enhanced = EnhancedToolError::new(error);
            assert_eq!(enhanced.category, ErrorCategory::Internal);
        }
    }

    #[test]
    fn test_recoverable_invalid_arguments() {
        let error = ToolError::InvalidArguments("test".to_string());
        let enhanced = EnhancedToolError::new(error);
        assert!(enhanced.recoverable);
    }

    #[test]
    fn test_not_recoverable_io_error() {
        let error = ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let enhanced = EnhancedToolError::new(error);
        assert!(!enhanced.recoverable);
    }

    #[test]
    fn test_not_recoverable_permission_denied() {
        let error = ToolError::PermissionDenied("denied".to_string());
        let enhanced = EnhancedToolError::new(error);
        assert!(!enhanced.recoverable);
    }

    #[test]
    fn test_recoverable_not_found() {
        let error = ToolError::NotFound("missing".to_string());
        let enhanced = EnhancedToolError::new(error);
        assert!(enhanced.recoverable);
    }

    #[test]
    fn test_recoverable_timeout() {
        let error = ToolError::Timeout;
        let enhanced = EnhancedToolError::new(error);
        assert!(enhanced.recoverable);
    }

    #[test]
    fn test_user_friendly_message() {
        let error = ToolError::NotFound("file.txt".to_string());
        let enhanced = EnhancedToolError::new(error)
            .with_context("path", "/test/file.txt")
            .with_suggestion("Check if the file exists");

        let message = enhanced.user_friendly_message();
        assert!(message.contains("file.txt"));
        assert!(message.contains("Context:"));
        assert!(message.contains("path"));
        assert!(message.contains("Suggestions:"));
        assert!(message.contains("Check if the file exists"));
        assert!(message.contains("recoverable"));
    }

    #[test]
    fn test_user_friendly_message_without_context() {
        let error = ToolError::Timeout;
        let enhanced = EnhancedToolError::new(error);

        let message = enhanced.user_friendly_message();
        // The message contains the emoji and category
        assert!(!message.is_empty());
        assert!(!message.contains("Context:"));
    }

    #[test]
    fn test_user_friendly_message_without_suggestions() {
        let error = ToolError::Cancelled;
        let enhanced = EnhancedToolError::new(error);

        let message = enhanced.user_friendly_message();
        assert!(!message.is_empty());
        assert!(!message.contains("Suggestions:"));
    }

    #[test]
    fn test_user_friendly_message_not_recoverable() {
        let error = ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "disk error"));
        let enhanced = EnhancedToolError::new(error);

        let message = enhanced.user_friendly_message();
        assert!(message.contains("manual intervention"));
    }

    #[test]
    fn test_to_json() {
        let error = ToolError::InvalidArguments("bad input".to_string());
        let enhanced = EnhancedToolError::new(error)
            .with_context("field", "username")
            .with_suggestion("Use alphanumeric characters");

        let json = enhanced.to_json();
        assert!(json["error"].as_str().unwrap().contains("bad input"));
        assert_eq!(json["error_type"], "InvalidArguments");
        assert_eq!(json["category"], "UserInput");
        assert_eq!(json["recoverable"], true);
        assert!(json["context"].is_object());
        assert!(json["suggestions"].is_array());
        assert!(json["timestamp"].is_string());
    }

    #[test]
    fn test_display_trait() {
        let error = ToolError::ValidationFailed("format error".to_string());
        let enhanced = EnhancedToolError::new(error);

        let display_string = format!("{}", enhanced);
        assert!(display_string.contains("format error"));
    }

    #[test]
    fn test_error_trait() {
        let error = ToolError::Other("test".to_string());
        let enhanced = EnhancedToolError::new(error);

        // Test that it implements std::error::Error
        let _err: &dyn std::error::Error = &enhanced;
        assert!(enhanced.source().is_none());
    }

    #[test]
    fn test_from_tool_error() {
        let tool_error = ToolError::NotFound("resource".to_string());
        let enhanced: EnhancedToolError = tool_error.into();

        assert_eq!(enhanced.original_error_type, "NotFound");
        assert!(enhanced.original_error_message.contains("resource"));
    }

    #[test]
    fn test_helper_file_not_found() {
        let error = helpers::file_not_found("/path/to/file.txt");

        assert_eq!(error.category, ErrorCategory::FileSystem);
        assert!(error.context.contains_key("file_path"));
        assert!(error.suggestions.len() > 0);
        assert!(error.original_error_message.contains("File not found"));
    }

    #[test]
    fn test_helper_permission_denied() {
        let error = helpers::permission_denied("write", "/secure/file");

        assert_eq!(error.category, ErrorCategory::Permission);
        assert!(error.context.contains_key("operation"));
        assert!(error.context.contains_key("resource"));
        assert!(error.suggestions.len() >= 3);
    }

    #[test]
    fn test_helper_invalid_argument() {
        let error = helpers::invalid_argument("port", "abc", "number between 1-65535");

        assert_eq!(error.category, ErrorCategory::UserInput);
        assert_eq!(error.context.get("parameter"), Some(&"port".to_string()));
        assert_eq!(error.context.get("provided_value"), Some(&"abc".to_string()));
        assert_eq!(error.context.get("expected_format"), Some(&"number between 1-65535".to_string()));
    }

    #[test]
    fn test_helper_timeout_error() {
        let error = helpers::timeout_error("network_request", 30);

        assert_eq!(error.category, ErrorCategory::Resource);
        assert_eq!(error.original_error_type, "Timeout");
        assert!(error.context.contains_key("operation"));
        assert!(error.context.contains_key("timeout_seconds"));
    }

    #[test]
    fn test_helper_configuration_error() {
        let error = helpers::configuration_error("api_key", "missing value");

        assert_eq!(error.category, ErrorCategory::Configuration);
        assert!(error.context.contains_key("config_key"));
        assert!(error.context.contains_key("issue"));
        assert!(error.suggestions.len() >= 3);
    }

    #[test]
    fn test_error_category_debug() {
        assert!(format!("{:?}", ErrorCategory::UserInput).contains("UserInput"));
        assert!(format!("{:?}", ErrorCategory::FileSystem).contains("FileSystem"));
        assert!(format!("{:?}", ErrorCategory::Network).contains("Network"));
        assert!(format!("{:?}", ErrorCategory::Permission).contains("Permission"));
        assert!(format!("{:?}", ErrorCategory::Configuration).contains("Configuration"));
        assert!(format!("{:?}", ErrorCategory::Resource).contains("Resource"));
        assert!(format!("{:?}", ErrorCategory::Dependency).contains("Dependency"));
        assert!(format!("{:?}", ErrorCategory::Internal).contains("Internal"));
    }

    #[test]
    fn test_error_category_clone() {
        let category = ErrorCategory::Network;
        let cloned = category.clone();
        assert_eq!(category, cloned);
    }

    #[test]
    fn test_error_category_partial_eq() {
        assert_eq!(ErrorCategory::UserInput, ErrorCategory::UserInput);
        assert_ne!(ErrorCategory::UserInput, ErrorCategory::FileSystem);
    }

    #[test]
    fn test_enhanced_error_debug() {
        let error = ToolError::NotFound("test".to_string());
        let enhanced = EnhancedToolError::new(error);

        let debug_string = format!("{:?}", enhanced);
        assert!(debug_string.contains("EnhancedToolError"));
    }

    #[test]
    fn test_get_error_type_all_variants() {
        let test_cases = vec![
            (ToolError::InvalidArguments("test".into()), "InvalidArguments"),
            (ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")), "Io"),
            (ToolError::PermissionDenied("test".into()), "PermissionDenied"),
            (ToolError::NotFound("test".into()), "NotFound"),
            (ToolError::Timeout, "Timeout"),
            (ToolError::Json(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "test"))), "Json"),
            (ToolError::ExecutionFailed("test".into()), "ExecutionFailed"),
            (ToolError::ValidationFailed("test".into()), "ValidationFailed"),
            (ToolError::Cancelled, "Cancelled"),
            (ToolError::Other("test".into()), "Other"),
        ];

        for (error, expected_type) in test_cases {
            let enhanced = EnhancedToolError::new(error);
            assert_eq!(enhanced.original_error_type, expected_type);
        }
    }

    #[test]
    fn test_chain_multiple_modifications() {
        let error = ToolError::InvalidArguments("test".to_string());
        let enhanced = EnhancedToolError::new(error)
            .with_context("key1", "value1")
            .with_context("key2", "value2")
            .with_suggestion("suggestion1")
            .with_suggestion("suggestion2")
            .with_category(ErrorCategory::Configuration)
            .with_recoverable(false);

        assert_eq!(enhanced.context.len(), 2);
        assert_eq!(enhanced.suggestions.len(), 2);
        assert_eq!(enhanced.category, ErrorCategory::Configuration);
        assert!(!enhanced.recoverable);
    }
}
