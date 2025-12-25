//! Error-related tests

#![cfg(test)]

use crate::error::SageError;
use crate::tools::base::concurrency::ConcurrencyMode;
use crate::tools::base::error::ToolError;

#[test]
fn test_tool_error_conversions() {
    // Test NotFound error
    let err = ToolError::NotFound("test_tool".to_string());
    let sage_err: SageError = err.into();
    assert!(sage_err.to_string().contains("Tool not found"));

    // Test InvalidArguments error
    let err = ToolError::InvalidArguments("invalid arg".to_string());
    let sage_err: SageError = err.into();
    assert!(sage_err.to_string().contains("invalid arg"));

    // Test Timeout error
    let err = ToolError::Timeout;
    let sage_err: SageError = err.into();
    assert!(sage_err.to_string().contains("timeout"));
}

#[test]
fn test_tool_error_display() {
    let err = ToolError::NotFound("test_tool".to_string());
    assert_eq!(err.to_string(), "Tool not found: test_tool");

    let err = ToolError::InvalidArguments("bad arg".to_string());
    assert_eq!(err.to_string(), "Invalid arguments: bad arg");

    let err = ToolError::Timeout;
    assert_eq!(err.to_string(), "Tool execution timeout");

    let err = ToolError::Cancelled;
    assert_eq!(err.to_string(), "Tool execution cancelled");
}

#[test]
fn test_concurrency_mode_equality() {
    assert_eq!(ConcurrencyMode::Parallel, ConcurrencyMode::Parallel);
    assert_eq!(ConcurrencyMode::Sequential, ConcurrencyMode::Sequential);
    assert_eq!(ConcurrencyMode::Limited(5), ConcurrencyMode::Limited(5));
    assert_ne!(ConcurrencyMode::Limited(5), ConcurrencyMode::Limited(10));
    assert_eq!(
        ConcurrencyMode::ExclusiveByType,
        ConcurrencyMode::ExclusiveByType
    );
}

#[test]
fn test_concurrency_mode_default() {
    let mode: ConcurrencyMode = Default::default();
    assert_eq!(mode, ConcurrencyMode::Parallel);
}
