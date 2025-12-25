//! Docker tool tests

use super::tool::DockerTool;
use sage_core::tools::Tool;

#[tokio::test]
async fn test_docker_tool_creation() {
    let tool = DockerTool::new();
    assert_eq!(tool.name(), "docker");
    assert!(!tool.description().is_empty());
}

#[tokio::test]
async fn test_docker_tool_schema() {
    let tool = DockerTool::new();
    let schema = tool.parameters_json_schema();

    assert!(schema.is_object());
    assert!(schema["properties"]["operation"].is_object());
}
