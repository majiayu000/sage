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
            message.push_str("\n🔄 This error may be recoverable. Please try the suggestions above.\n");
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
        EnhancedToolError::new(ToolError::NotFound(format!("File not found: {}", file_path)))
            .with_context("file_path", file_path)
            .with_suggestion("Check if the file path is correct")
            .with_suggestion("Ensure the file exists in the specified location")
            .with_suggestion("Verify you have read permissions for the file")
            .with_category(ErrorCategory::FileSystem)
    }

    /// Create an enhanced permission denied error
    pub fn permission_denied(operation: &str, resource: &str) -> EnhancedToolError {
        EnhancedToolError::new(ToolError::PermissionDenied(format!(
            "Permission denied: cannot {} {}", operation, resource
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
            "Invalid argument '{}': got '{}', expected {}", parameter, value, expected
        )))
            .with_context("parameter", parameter)
            .with_context("provided_value", value)
            .with_context("expected_format", expected)
            .with_suggestion(format!("Provide a valid value for parameter '{}'", parameter))
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
            "Configuration error for '{}': {}", config_key, issue
        )))
            .with_context("config_key", config_key)
            .with_context("issue", issue)
            .with_suggestion("Check the configuration file")
            .with_suggestion("Verify the configuration value format")
            .with_suggestion("Reset to default configuration if needed")
            .with_category(ErrorCategory::Configuration)
    }
}
