//! Error categorization and context analysis

use super::types::ErrorCategory;
use sage_core::tools::base::ToolError;

/// Get error type as string
pub fn get_error_type(error: &ToolError) -> String {
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
        ToolError::ConfirmationRequired(_) => "ConfirmationRequired".to_string(),
        ToolError::Other(_) => "Other".to_string(),
    }
}

/// Categorize error based on its type
pub fn categorize_error(error: &ToolError) -> ErrorCategory {
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
        ToolError::ConfirmationRequired(_) => ErrorCategory::UserInput,
        ToolError::Other(_) => ErrorCategory::Internal,
    }
}

/// Determine if error is recoverable
pub fn is_recoverable(error: &ToolError) -> bool {
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
        ToolError::ConfirmationRequired(_) => true,
        ToolError::Other(_) => false,
    }
}
