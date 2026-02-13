//! User-friendly error messages and classification
//!
//! Provides human-readable error messages and suggestions for common error scenarios.
//! Similar to Claude Code's error handling with `userMessage` field.

use super::classifiers::{classify_config_error, classify_http_error, classify_llm_error};
use super::types::SageError;

/// Error category for user-facing messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Configuration issues
    Configuration,
    /// Authentication/authorization failures
    Authentication,
    /// Rate limiting / quota exceeded
    RateLimit,
    /// Network connectivity issues
    Network,
    /// Invalid user input
    UserInput,
    /// Tool execution failures
    ToolExecution,
    /// Internal system errors
    Internal,
    /// Resource not available
    ResourceUnavailable,
    /// User-initiated cancellation
    Cancellation,
    /// File system related errors
    FileSystem,
    /// Permission and security errors
    Permission,
    /// System resource errors (memory, disk, timeout, etc.)
    Resource,
    /// External dependency errors
    Dependency,
}

impl ErrorCategory {
    /// Get a user-friendly category name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Configuration => "Configuration Error",
            Self::Authentication => "Authentication Error",
            Self::RateLimit => "Rate Limit Exceeded",
            Self::Network => "Network Error",
            Self::UserInput => "Invalid Input",
            Self::ToolExecution => "Tool Execution Error",
            Self::Internal => "Internal Error",
            Self::ResourceUnavailable => "Resource Unavailable",
            Self::Cancellation => "Cancelled",
            Self::FileSystem => "File System Error",
            Self::Permission => "Permission Error",
            Self::Resource => "Resource Error",
            Self::Dependency => "Dependency Error",
        }
    }
}

/// User-friendly error information
#[derive(Debug, Clone)]
pub struct UserFriendlyError {
    /// The error category
    pub category: ErrorCategory,
    /// User-friendly title/summary
    pub title: String,
    /// Detailed user-friendly message
    pub message: String,
    /// Suggested actions to resolve the error
    pub suggestions: Vec<String>,
    /// Whether this error is recoverable by the user
    pub is_recoverable: bool,
    /// Original technical error code
    pub error_code: String,
}

impl UserFriendlyError {
    /// Create a new user-friendly error
    pub fn new(
        category: ErrorCategory,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            category,
            title: title.into(),
            message: message.into(),
            suggestions: Vec::new(),
            is_recoverable: true,
            error_code: String::new(),
        }
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

    /// Set whether the error is recoverable
    pub fn recoverable(mut self, is_recoverable: bool) -> Self {
        self.is_recoverable = is_recoverable;
        self
    }

    /// Set the technical error code
    pub fn with_error_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = code.into();
        self
    }

    /// Format the error for display
    pub fn format_display(&self) -> String {
        let mut output = format!(
            "{}: {}\n\n{}",
            self.category.display_name(),
            self.title,
            self.message
        );

        if !self.suggestions.is_empty() {
            output.push_str("\n\nSuggested actions:");
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                output.push_str(&format!("\n  {}. {}", i + 1, suggestion));
            }
        }

        output
    }
}

/// Convert SageError to user-friendly error
impl From<&SageError> for UserFriendlyError {
    fn from(error: &SageError) -> Self {
        match error {
            SageError::Config { message, .. } => {
                let (title, suggestions) = classify_config_error(message);
                UserFriendlyError::new(ErrorCategory::Configuration, title, message.clone())
                    .with_suggestions(suggestions)
                    .with_error_code("SAGE_CONFIG")
            }

            SageError::Llm {
                message, provider, ..
            } => {
                let (category, title, suggestions) =
                    classify_llm_error(message, provider.as_deref());
                UserFriendlyError::new(category, title, message.clone())
                    .with_suggestions(suggestions)
                    .with_error_code("SAGE_LLM")
            }

            SageError::Http {
                message,
                status_code,
                url,
                ..
            } => {
                let (category, title, suggestions) =
                    classify_http_error(message, *status_code, url.as_deref());
                UserFriendlyError::new(category, title, message.clone())
                    .with_suggestions(suggestions)
                    .with_error_code("SAGE_HTTP")
            }

            SageError::Tool {
                tool_name, message, ..
            } => UserFriendlyError::new(
                ErrorCategory::ToolExecution,
                format!("Tool '{}' failed", tool_name),
                message.clone(),
            )
            .with_suggestion(format!(
                "Check if the tool '{}' is available and properly configured",
                tool_name
            ))
            .with_suggestion("Review the tool arguments for correctness".to_string())
            .with_error_code("SAGE_TOOL"),

            SageError::InvalidInput { message, field, .. } => {
                let title = if let Some(f) = field {
                    format!("Invalid value for '{}'", f)
                } else {
                    "Invalid input".to_string()
                };
                UserFriendlyError::new(ErrorCategory::UserInput, title, message.clone())
                    .with_suggestion("Check the input format and try again".to_string())
                    .with_error_code("SAGE_INVALID_INPUT")
            }

            SageError::Timeout { seconds, .. } => UserFriendlyError::new(
                ErrorCategory::Internal,
                "Operation timed out",
                format!("The operation did not complete within {} seconds", seconds),
            )
            .with_suggestion("Try again with a simpler request".to_string())
            .with_suggestion("Consider breaking the task into smaller steps".to_string())
            .with_error_code("SAGE_TIMEOUT"),

            SageError::Cancelled => UserFriendlyError::new(
                ErrorCategory::Cancellation,
                "Operation cancelled",
                "The operation was cancelled by user request".to_string(),
            )
            .recoverable(false)
            .with_error_code("SAGE_CANCELLED"),

            SageError::NotFound {
                message,
                resource_type,
                ..
            } => {
                let title = if let Some(rt) = resource_type {
                    format!("{} not found", rt)
                } else {
                    "Resource not found".to_string()
                };
                UserFriendlyError::new(ErrorCategory::ResourceUnavailable, title, message.clone())
                    .with_suggestion("Verify the path or identifier is correct".to_string())
                    .with_error_code("SAGE_NOT_FOUND")
            }

            SageError::Io { message, path, .. } => {
                let title = if path.is_some() {
                    "File operation failed"
                } else {
                    "I/O error"
                };
                UserFriendlyError::new(ErrorCategory::Internal, title, message.clone())
                    .with_suggestion("Check file permissions".to_string())
                    .with_suggestion("Verify the path exists".to_string())
                    .with_error_code("SAGE_IO")
            }

            SageError::Json { message, .. } => UserFriendlyError::new(
                ErrorCategory::Internal,
                "Data format error",
                message.clone(),
            )
            .with_suggestion("The data may be corrupted or in an unexpected format".to_string())
            .with_error_code("SAGE_JSON"),

            SageError::Agent { message, .. } => UserFriendlyError::new(
                ErrorCategory::Internal,
                "Agent execution error",
                message.clone(),
            )
            .with_suggestion("Try rephrasing your request".to_string())
            .with_error_code("SAGE_AGENT"),

            SageError::Cache { message, .. } => {
                UserFriendlyError::new(ErrorCategory::Internal, "Cache error", message.clone())
                    .with_suggestion("Try clearing the cache and retrying".to_string())
                    .with_error_code("SAGE_CACHE")
            }

            SageError::Storage { message, .. } => {
                UserFriendlyError::new(ErrorCategory::Internal, "Storage error", message.clone())
                    .with_suggestion("Check disk space and permissions".to_string())
                    .with_error_code("SAGE_STORAGE")
            }

            SageError::Other { message, .. } => {
                UserFriendlyError::new(ErrorCategory::Internal, "Unexpected error", message.clone())
                    .with_suggestion("If this persists, please report the issue".to_string())
                    .with_error_code("SAGE_OTHER")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_friendly_error_from_config() {
        let error = SageError::Config {
            message: "Configuration file not found".to_string(),
            source: None,
            context: None,
        };

        let friendly: UserFriendlyError = (&error).into();
        assert_eq!(friendly.category, ErrorCategory::Configuration);
        assert!(!friendly.suggestions.is_empty());
    }

    #[test]
    fn test_user_friendly_error_from_llm_auth() {
        let error = SageError::Llm {
            message: "401 Unauthorized - Invalid API key".to_string(),
            provider: Some("Anthropic".to_string()),
            context: None,
        };

        let friendly: UserFriendlyError = (&error).into();
        assert_eq!(friendly.category, ErrorCategory::Authentication);
    }

    #[test]
    fn test_user_friendly_error_from_rate_limit() {
        let error = SageError::Http {
            message: "Rate limit exceeded".to_string(),
            url: None,
            status_code: Some(429),
            context: None,
        };

        let friendly: UserFriendlyError = (&error).into();
        assert_eq!(friendly.category, ErrorCategory::RateLimit);
    }

    #[test]
    fn test_format_display() {
        let error = UserFriendlyError::new(
            ErrorCategory::Authentication,
            "API key invalid",
            "The provided API key was rejected",
        )
        .with_suggestion("Check your API key".to_string())
        .with_suggestion("Regenerate the key if needed".to_string());

        let display = error.format_display();
        assert!(display.contains("Authentication Error"));
        assert!(display.contains("API key invalid"));
        assert!(display.contains("Suggested actions"));
        assert!(display.contains("1. Check your API key"));
        assert!(display.contains("2. Regenerate the key"));
    }
}
