//! Helper functions for common error scenarios

use super::types::EnhancedToolError;
use sage_core::error::ErrorCategory;
use sage_core::tools::base::ToolError;

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
