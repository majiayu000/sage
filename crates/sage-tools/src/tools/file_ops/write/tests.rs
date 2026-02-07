//! Test suite for WriteTool

#[cfg(test)]
mod suite {
    use crate::tools::file_ops::write::WriteTool;
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
    async fn test_write_tool_create_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-1",
            "Write",
            json!({
                "file_path": "test.txt",
                "content": "Hello, World!"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("created"));

        // Verify the file was created with correct content
        let file_path = temp_dir.path().join("test.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_tool_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-2",
            "Write",
            json!({
                "file_path": "subdir/nested/test.txt",
                "content": "Nested file content"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify the file was created in nested directories
        let file_path = temp_dir.path().join("subdir/nested/test.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Nested file content");
    }

    #[tokio::test]
    async fn test_write_tool_overwrite_after_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        fs::write(&file_path, "Initial content").await.unwrap();

        let tool = WriteTool::with_working_directory(temp_dir.path());

        // Mark file as read
        tool.mark_file_as_read(file_path.clone());

        let call = create_tool_call(
            "test-3",
            "Write",
            json!({
                "file_path": "test.txt",
                "content": "Updated content"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("overwritten"));

        // Verify the file was overwritten
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Updated content");
    }

    #[tokio::test]
    async fn test_write_tool_overwrite_without_read_fails() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        fs::write(&file_path, "Initial content").await.unwrap();

        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-4",
            "Write",
            json!({
                "file_path": "test.txt",
                "content": "Attempting to overwrite"
            }),
        );

        // Should fail because file exists but hasn't been read
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(sage_core::tools::base::ToolError::ValidationFailed(msg)) => {
                assert!(msg.contains("has not been read"));
            }
            _ => panic!("Expected ValidationFailed error"),
        }

        // Verify original content is unchanged
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Initial content");
    }

    #[tokio::test]
    async fn test_write_tool_missing_parameters() {
        let tool = WriteTool::new();

        // Missing file_path
        let call = create_tool_call(
            "test-5a",
            "Write",
            json!({
                "content": "Some content"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());

        // Missing content
        let call = create_tool_call(
            "test-5b",
            "Write",
            json!({
                "file_path": "test.txt"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_tool_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-6",
            "Write",
            json!({
                "file_path": "empty.txt",
                "content": ""
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify empty file was created
        let file_path = temp_dir.path().join("empty.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "");
    }

    #[tokio::test]
    async fn test_write_tool_multiline_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        let multiline_content = "Line 1\nLine 2\nLine 3\n";
        let call = create_tool_call(
            "test-7",
            "Write",
            json!({
                "file_path": "multiline.txt",
                "content": multiline_content
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify multiline content
        let file_path = temp_dir.path().join("multiline.txt");
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, multiline_content);
    }

    #[tokio::test]
    async fn test_write_tool_binary_safe_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::with_working_directory(temp_dir.path());

        // Content with special characters
        let content = "Special chars: \t\r\n\0";
        let call = create_tool_call(
            "test-8",
            "Write",
            json!({
                "file_path": "special.txt",
                "content": content
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Verify content with special characters
        let file_path = temp_dir.path().join("special.txt");
        let read_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_write_tool_schema() {
        let tool = WriteTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Write");
        assert!(!schema.description.is_empty());

        // Verify schema has required parameters
        if let serde_json::Value::Object(params) = &schema.parameters {
            if let Some(serde_json::Value::Object(properties)) = params.get("properties") {
                assert!(properties.contains_key("file_path"));
                assert!(properties.contains_key("content"));
            }
        }
    }

    #[test]
    fn test_write_tool_validation() {
        let tool = WriteTool::new();

        // Valid call
        let call = create_tool_call(
            "test-9",
            "Write",
            json!({
                "file_path": "/absolute/path/test.txt",
                "content": "Valid content"
            }),
        );
        assert!(tool.validate(&call).is_ok());

        // Invalid - missing parameters
        let call = create_tool_call(
            "test-10",
            "Write",
            json!({
                "file_path": "/path/test.txt"
            }),
        );
        assert!(tool.validate(&call).is_err());
    }
}
