//! Tests for security scanner

use super::tool::SecurityScannerTool;
use sage_core::tools::Tool;

#[tokio::test]
async fn test_security_scanner_tool_creation() {
    let tool = SecurityScannerTool::new();
    assert_eq!(tool.name(), "security_scanner");
    assert!(!tool.description().is_empty());
}

#[tokio::test]
async fn test_security_scanner_schema() {
    let tool = SecurityScannerTool::new();
    let schema = tool.parameters_json_schema();

    assert!(schema.is_object());
    assert!(schema["properties"]["operation"].is_object());
}
