//! Enhanced error types for Sage Tools

use sage_core::error::ErrorCategory;
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

impl EnhancedToolError {
    /// Create a new enhanced error from a tool error
    pub fn new(original_error: ToolError) -> Self {
        use crate::tools::utils::enhanced_errors::context::{
            categorize_error, get_error_type, is_recoverable,
        };

        let original_error_message = original_error.to_string();
        let original_error_type = get_error_type(&original_error);
        let category = categorize_error(&original_error);
        let recoverable = is_recoverable(&original_error);

        Self {
            original_error_message,
            original_error_type,
            context: HashMap::new(),
            suggestions: Vec::new(),
            category,
            recoverable,
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
