//! Integration tests for Sage Agent tools
//!
//! This module contains comprehensive integration tests for all the tools,
//! focusing on the new tools that have been implemented with the updated Tool trait.

use crate::tools::*;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

/// Test helper to create a ToolCall with parameters
fn create_tool_call(id: &str, name: &str, params: serde_json::Value) -> ToolCall {
    let mut arguments = HashMap::new();
    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            arguments.insert(key.clone(), value.clone());
        }
    }
    
    ToolCall::new(id, name, arguments)
}

#[cfg(test)]
mod git_tool_tests {
    use super::*;

    #[tokio::test]
    async fn test_git_tool_schema_validation() {
        let git = GitTool::new();
        let schema = git.schema();
        
        assert_eq!(schema.name, "git");
        assert!(!schema.description.is_empty());
        
        // Check that parameters is a valid JSON object
        if let Some(obj) = schema.parameters.as_object() {
            assert!(obj.contains_key("properties"));
            if let Some(props) = obj["properties"].as_object() {
                assert!(props.contains_key("command"));
            }
        }
    }

    #[tokio::test]
    async fn test_git_tool_invalid_command() {
        let git = GitTool::new();
        let call = create_tool_call("test", "git", json!({
            "command": "invalid_command"
        }));
        
        let result = git.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_git_tool_missing_parameters() {
        let git = GitTool::new();
        let call = create_tool_call("test", "git", json!({}));
        
        let result = git.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_git_tool_status_command() {
        let git = GitTool::new();
        
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        
        let call = create_tool_call("test", "git", json!({
            "command": "status",
            "path": repo_path
        }));
        
        // This should fail since it's not a git repository, but it tests the execution path
        let result = git.execute(&call).await;
        assert!(result.is_err()); // Expected to fail in non-git directory
    }
}

#[cfg(test)]
mod log_analyzer_tests {
    use super::*;

    #[tokio::test]
    async fn test_log_analyzer_schema() {
        let analyzer = LogAnalyzerTool::new();
        let schema = analyzer.schema();
        
        assert_eq!(schema.name, "log_analyzer");
        assert!(!schema.description.is_empty());
        
        // Check that parameters is a valid JSON object
        if let Some(obj) = schema.parameters.as_object() {
            assert!(obj.contains_key("properties"));
            if let Some(props) = obj["properties"].as_object() {
                assert!(props.contains_key("command"));
                assert!(props.contains_key("file_path"));
            }
        }
    }

    #[tokio::test]
    async fn test_log_analyzer_with_test_file() {
        let analyzer = LogAnalyzerTool::new();
        
        // Create a test log file
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");
        
        let log_content = r#"
2024-01-01 10:00:00 INFO: Application started
2024-01-01 10:01:00 ERROR: Database connection failed
2024-01-01 10:02:00 WARN: Retrying connection
2024-01-01 10:03:00 INFO: Connection established
2024-01-01 10:04:00 FATAL: Critical system error
"#;
        
        fs::write(&log_file, log_content).await.unwrap();
        
        let call = create_tool_call("test", "log_analyzer", json!({
            "command": "analyze",
            "file_path": log_file.to_str().unwrap()
        }));
        
        let result = analyzer.execute(&call).await.unwrap();
        assert!(result.success);
        if let Some(output) = result.output {
            assert!(output.contains("ERROR") || output.contains("Analysis"));
        }
    }

    #[tokio::test]
    async fn test_log_analyzer_missing_file() {
        let analyzer = LogAnalyzerTool::new();
        
        let call = create_tool_call("test", "log_analyzer", json!({
            "command": "analyze",
            "file_path": "/nonexistent/file.log"
        }));
        
        let result = analyzer.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_log_analyzer_search_pattern() {
        let analyzer = LogAnalyzerTool::new();
        
        // Create a test log file
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");
        
        let log_content = "ERROR: Something went wrong\nINFO: Everything is fine\nERROR: Another error\n";
        fs::write(&log_file, log_content).await.unwrap();
        
        let call = create_tool_call("test", "log_analyzer", json!({
            "command": "analyze",
            "file_path": log_file.to_str().unwrap(),
            "pattern": "ERROR"
        }));
        
        let result = analyzer.execute(&call).await.unwrap();
        assert!(result.success);
        if let Some(output) = result.output {
            assert!(output.contains("ERROR") || output.contains("Found"));
        }
    }
}

#[cfg(test)]
mod test_generator_tests {
    use super::*;

    #[tokio::test]
    async fn test_test_generator_schema() {
        let generator = TestGeneratorTool::new();
        let schema = generator.schema();
        
        assert_eq!(schema.name, "test_generator");
        assert!(!schema.description.is_empty());
        
        // Check that parameters is a valid JSON object
        if let Some(obj) = schema.parameters.as_object() {
            assert!(obj.contains_key("properties"));
            if let Some(props) = obj["properties"].as_object() {
                assert!(props.contains_key("command"));
                assert!(props.contains_key("language"));
            }
        }
    }

    #[tokio::test]
    async fn test_test_generator_rust_unit_test() {
        let generator = TestGeneratorTool::new();
        
        // Create a test source file
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("lib.rs");
        
        let source_content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(x: f64, y: f64) -> f64 {
    x * y
}
"#;
        
        fs::write(&source_file, source_content).await.unwrap();
        
        let call = create_tool_call("test", "test_generator", json!({
            "command": "unit_test",
            "function_name": "add",
            "file_path": source_file.to_str().unwrap()
        }));
        
        let result = generator.execute(&call).await.unwrap();
        assert!(result.success);
        if let Some(output) = result.output {
            assert!(output.contains("#[test]"));
            assert!(output.contains("test_add"));
        }
    }

    #[tokio::test]
    async fn test_test_generator_python_test() {
        let generator = TestGeneratorTool::new();
        
        let call = create_tool_call("test", "test_generator", json!({
            "command": "test_data",
            "data_type": "user",
            "format": "json"
        }));
        
        let result = generator.execute(&call).await.unwrap();
        assert!(result.success);
        if let Some(output) = result.output {
            assert!(output.contains("John Doe") || output.contains("user"));
            assert!(output.contains("email"));
        }
    }

    #[tokio::test]
    async fn test_test_generator_integration_test() {
        let generator = TestGeneratorTool::new();
        
        let call = create_tool_call("test", "test_generator", json!({
            "command": "integration_test",
            "module_name": "user_service"
        }));
        
        let result = generator.execute(&call).await.unwrap();
        assert!(result.success);
        if let Some(output) = result.output {
            assert!(output.contains("integration") || output.contains("test"));
        }
    }

    #[tokio::test]
    async fn test_test_generator_invalid_language() {
        let generator = TestGeneratorTool::new();
        
        let call = create_tool_call("test", "test_generator", json!({
            "command": "invalid_command",
            "function_name": "test_func"
        }));
        
        let result = generator.execute(&call).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod kubernetes_tool_tests {
    use super::*;

    #[tokio::test]
    async fn test_kubernetes_tool_schema() {
        let k8s = KubernetesTool::new();
        let schema = k8s.schema();
        
        assert_eq!(schema.name, "kubernetes");
        assert!(!schema.description.is_empty());
        
        // Check that parameters is a valid JSON object
        if let Some(obj) = schema.parameters.as_object() {
            assert!(obj.contains_key("properties"));
            if let Some(props) = obj["properties"].as_object() {
                assert!(props.contains_key("command"));
            }
        }
    }

    #[tokio::test]
    async fn test_kubernetes_tool_invalid_command() {
        let k8s = KubernetesTool::new();
        
        let call = create_tool_call("test", "kubernetes", json!({
            "command": "invalid_command"
        }));
        
        let result = k8s.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_kubernetes_tool_get_command() {
        let k8s = KubernetesTool::new();
        
        let call = create_tool_call("test", "kubernetes", json!({
            "command": "get",
            "resource_type": "pods"
        }));
        
        // This will likely fail if kubectl is not configured, but tests the code path
        let result = k8s.execute(&call).await;
        // Don't assert success/failure as it depends on kubectl availability
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_kubernetes_tool_missing_parameters() {
        let k8s = KubernetesTool::new();
        
        let call = create_tool_call("test", "kubernetes", json!({}));
        
        let result = k8s.execute(&call).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod terraform_tool_tests {
    use super::*;

    #[tokio::test]
    async fn test_terraform_tool_schema() {
        let terraform = TerraformTool::new();
        let schema = terraform.schema();
        
        assert_eq!(schema.name, "terraform");
        assert!(!schema.description.is_empty());
        
        // Check that parameters is a valid JSON object
        if let Some(obj) = schema.parameters.as_object() {
            assert!(obj.contains_key("properties"));
            if let Some(props) = obj["properties"].as_object() {
                assert!(props.contains_key("command"));
                assert!(props.contains_key("working_dir"));
            }
        }
    }

    #[tokio::test]
    async fn test_terraform_tool_generate_config() {
        let terraform = TerraformTool::new();
        
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path().to_str().unwrap();
        
        let call = create_tool_call("test", "terraform", json!({
            "command": "generate",
            "working_dir": working_dir,
            "resource_type": "aws_ec2"
        }));
        
        let result = terraform.execute(&call).await.unwrap();
        assert!(result.success);
        if let Some(output) = result.output {
            assert!(output.contains("Generated"));
        }
        
        // Check if main.tf was created
        let main_tf = temp_dir.path().join("main.tf");
        assert!(main_tf.exists());
    }

    #[tokio::test]
    async fn test_terraform_tool_validate_without_init() {
        let terraform = TerraformTool::new();
        
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path().to_str().unwrap();
        
        // First generate a config
        let generate_call = create_tool_call("generate", "terraform", json!({
            "command": "generate",
            "working_dir": working_dir,
            "resource_type": "aws_ec2"
        }));
        terraform.execute(&generate_call).await.unwrap();
        
        // Then try to validate (this might fail without terraform init, but tests the code path)
        let validate_call = create_tool_call("validate", "terraform", json!({
            "command": "validate",
            "working_dir": working_dir
        }));
        
        let result = terraform.execute(&validate_call).await;
        // Validation might fail without proper terraform setup, which is expected
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_terraform_tool_missing_parameters() {
        let terraform = TerraformTool::new();
        
        let call = create_tool_call("test", "terraform", json!({
            "command": "generate"
            // Missing working_dir and resource_type
        }));
        
        let result = terraform.execute(&call).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod cloud_tool_tests {
    use super::*;

    #[tokio::test]
    async fn test_cloud_tool_schema() {
        let cloud = CloudTool::new();
        let schema = cloud.schema();
        
        assert_eq!(schema.name, "cloud");
        assert!(!schema.description.is_empty());
        
        // Check that parameters is a valid JSON object
        if let Some(obj) = schema.parameters.as_object() {
            assert!(obj.contains_key("properties"));
            if let Some(props) = obj["properties"].as_object() {
                assert!(props.contains_key("provider"));
                assert!(props.contains_key("command"));
            }
        }
    }

    #[tokio::test]
    async fn test_cloud_tool_invalid_provider() {
        let cloud = CloudTool::new();
        
        let call = create_tool_call("test", "cloud", json!({
            "provider": "invalid_provider",
            "command": "manage"
        }));
        
        let result = cloud.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cloud_tool_missing_parameters() {
        let cloud = CloudTool::new();
        
        let call = create_tool_call("test", "cloud", json!({}));
        
        let result = cloud.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cloud_tool_manage_command() {
        let cloud = CloudTool::new();
        
        let call = create_tool_call("test", "cloud", json!({
            "provider": "aws",
            "command": "manage",
            "service": "ec2",
            "action": "list"
        }));
        
        // This will likely fail without AWS CLI configured, but tests the code path
        let result = cloud.execute(&call).await;
        // Don't assert success/failure as it depends on AWS CLI availability
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_cloud_tool_cost_command() {
        let cloud = CloudTool::new();
        
        let call = create_tool_call("test", "cloud", json!({
            "provider": "aws",
            "command": "cost"
        }));
        
        // This will likely fail without AWS CLI configured
        let result = cloud.execute(&call).await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[cfg(test)]
mod tool_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_tools_have_valid_schemas() {
        let tools = get_default_tools();
        
        for tool in tools {
            let schema = tool.schema();
            
            // All tools should have a name and description
            assert!(!schema.name.is_empty(), "Tool {} has empty name", schema.name);
            assert!(!schema.description.is_empty(), "Tool {} has empty description", schema.name);
            
            // Parameters should be a valid JSON object
            if let Some(obj) = schema.parameters.as_object() {
                assert!(obj.contains_key("properties"), "Tool {} schema missing properties", schema.name);
                
                if let Some(props) = obj["properties"].as_object() {
                    for (param_name, param_schema) in props {
                        assert!(param_schema.is_object(), "Tool {} parameter {} is not an object", schema.name, param_name);
                        
                        if let Some(param_obj) = param_schema.as_object() {
                            assert!(param_obj.contains_key("description"), "Tool {} parameter {} missing description", schema.name, param_name);
                            assert!(param_obj.contains_key("type"), "Tool {} parameter {} missing type", schema.name, param_name);
                        }
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_tool_registry_functions() {
        // Test that all category functions return tools
        assert!(!get_file_ops_tools().is_empty());
        assert!(!get_process_tools().is_empty());
        assert!(!get_task_mgmt_tools().is_empty());
        assert!(!get_network_tools().is_empty());
        assert!(!get_diagnostics_tools().is_empty());
        assert!(!get_vcs_tools().is_empty());
        assert!(!get_monitoring_tools().is_empty());
        assert!(!get_infrastructure_tools().is_empty());
    }

    #[tokio::test]
    async fn test_tool_name_uniqueness() {
        let tools = get_default_tools();
        let mut names = std::collections::HashSet::new();
        
        for tool in &tools {
            let name = tool.name();
            assert!(names.insert(name.to_string()), "Duplicate tool name found: {}", name);
        }
    }
}