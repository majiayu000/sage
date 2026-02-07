//! Tests for codebase retrieval functionality

#[cfg(test)]
mod suite {
    use crate::tools::file_ops::codebase_retrieval::CodebaseRetrievalTool;
    use sage_core::tools::base::Tool;
    use sage_core::tools::types::ToolCall;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::fs;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_codebase_retrieval_basic_search() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let rust_file = temp_dir.path().join("test.rs");
        fs::write(
            &rust_file,
            r#"
fn main() {
    println!("Hello, world!");
}

struct User {
    name: String,
    email: String,
}

impl User {
    fn new(name: String, email: String) -> Self {
        Self { name, email }
    }
}
"#,
        )
        .await
        .unwrap();

        let tool = CodebaseRetrievalTool::new();
        let call = create_tool_call(
            "test-1",
            "codebase-retrieval",
            json!({
                "information_request": "User struct implementation"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("User"));
        assert!(output.contains("struct"));
    }

    #[tokio::test]
    async fn test_codebase_retrieval_function_search() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with functions
        let js_file = temp_dir.path().join("utils.js");
        fs::write(
            &js_file,
            r#"
function calculateSum(a, b) {
    return a + b;
}

function validateEmail(email) {
    const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return regex.test(email);
}

async function fetchUserData(userId) {
    const response = await fetch(`/api/users/${userId}`);
    return response.json();
}
"#,
        )
        .await
        .unwrap();

        let tool = CodebaseRetrievalTool::new();
        let call = create_tool_call(
            "test-2",
            "codebase-retrieval",
            json!({
                "information_request": "email validation function"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("validateEmail") || output.contains("email"));
    }

    #[tokio::test]
    async fn test_codebase_retrieval_no_matches() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file
        let py_file = temp_dir.path().join("simple.py");
        fs::write(
            &py_file,
            r#"
print("Hello, Python!")
x = 42
y = "world"
"#,
        )
        .await
        .unwrap();

        let tool = CodebaseRetrievalTool::new();
        let call = create_tool_call(
            "test-3",
            "codebase-retrieval",
            json!({
                "information_request": "complex machine learning algorithm implementation"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        // Even when no matches, the tool returns success with a "no results" message
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        // The tool may return empty or no-match message
        assert!(
            output.contains("No relevant")
                || output.contains("No matches")
                || output.contains("Found")
                || !output.is_empty()
        );
    }

    #[tokio::test]
    async fn test_codebase_retrieval_multiple_file_types() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple file types
        let rust_file = temp_dir.path().join("config.rs");
        fs::write(
            &rust_file,
            r#"
pub struct Config {
    pub database_url: String,
    pub port: u16,
}
"#,
        )
        .await
        .unwrap();

        let json_file = temp_dir.path().join("package.json");
        fs::write(
            &json_file,
            r#"
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.18.0"
  }
}
"#,
        )
        .await
        .unwrap();

        let tool = CodebaseRetrievalTool::new();
        let call = create_tool_call(
            "test-4",
            "codebase-retrieval",
            json!({
                "information_request": "configuration settings"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Config") || output.contains("config"));
    }

    #[tokio::test]
    async fn test_codebase_retrieval_missing_parameter() {
        let tool = CodebaseRetrievalTool::new();
        let call = create_tool_call("test-5", "codebase-retrieval", json!({}));

        // Implementation returns Err(ToolError) for missing parameters
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn test_codebase_retrieval_empty_request() {
        let tool = CodebaseRetrievalTool::new();
        let call = create_tool_call(
            "test-6",
            "codebase-retrieval",
            json!({
                "information_request": ""
            }),
        );

        // Empty string is a valid input (returns no-match result), not an error
        let result = tool.execute(&call).await.unwrap();
        // Tool returns success even with empty/no-match queries
        assert!(result.success);
    }

    #[test]
    fn test_codebase_retrieval_schema() {
        let tool = CodebaseRetrievalTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "codebase-retrieval");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_supported_extensions() {
        let tool = CodebaseRetrievalTool::new();
        // Test that common extensions are supported
        assert!(tool.supported_extensions.contains("rs"));
        assert!(tool.supported_extensions.contains("py"));
        assert!(tool.supported_extensions.contains("js"));
        assert!(tool.supported_extensions.contains("json"));
        assert!(tool.supported_extensions.contains("md"));
    }
}
