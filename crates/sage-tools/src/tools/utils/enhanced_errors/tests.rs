//! Tests for enhanced error handling

#[cfg(test)]
mod tests {
    use crate::tools::utils::enhanced_errors::{EnhancedToolError, ErrorCategory, helpers};
    use sage_core::tools::base::ToolError;
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
        assert_eq!(
            enhanced.context.get("file_path"),
            Some(&"/test/path".to_string())
        );
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
        let enhanced = EnhancedToolError::new(tool_error).with_category(ErrorCategory::Network);

        assert_eq!(enhanced.category, ErrorCategory::Network);
    }

    #[test]
    fn test_enhanced_tool_error_with_recoverable() {
        let tool_error = ToolError::Timeout;
        let enhanced = EnhancedToolError::new(tool_error).with_recoverable(false);

        assert!(!enhanced.recoverable);
    }

    #[test]
    fn test_error_category_user_input() {
        let errors = vec![
            ToolError::InvalidArguments("test".to_string()),
            ToolError::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "test",
            ))),
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
        assert_eq!(
            error.context.get("provided_value"),
            Some(&"abc".to_string())
        );
        assert_eq!(
            error.context.get("expected_format"),
            Some(&"number between 1-65535".to_string())
        );
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
            (
                ToolError::InvalidArguments("test".into()),
                "InvalidArguments",
            ),
            (
                ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")),
                "Io",
            ),
            (
                ToolError::PermissionDenied("test".into()),
                "PermissionDenied",
            ),
            (ToolError::NotFound("test".into()), "NotFound"),
            (ToolError::Timeout, "Timeout"),
            (
                ToolError::Json(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "test",
                ))),
                "Json",
            ),
            (ToolError::ExecutionFailed("test".into()), "ExecutionFailed"),
            (
                ToolError::ValidationFailed("test".into()),
                "ValidationFailed",
            ),
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
