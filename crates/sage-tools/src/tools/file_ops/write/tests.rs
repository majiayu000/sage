//! Test suite for WriteTool

#[cfg(test)]
mod suite {
    use crate::tools::file_ops::write::WriteTool;
    use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
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
    async fn test_write_tool_rejects_missing_parent_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("work");
        fs::create_dir(&workspace).await.unwrap();
        let tool = WriteTool::with_working_directory(&workspace);

        let result = tool
            .write_file("missing/../../outside.txt", "escaped")
            .await;

        assert!(
            !temp_dir.path().join("outside.txt").exists(),
            "Write must not create a file outside the workspace; result: {result:?}"
        );
        assert!(matches!(result, Err(ToolError::PermissionDenied(_))));
        assert!(!workspace.join("missing").exists());
        assert!(!tool.is_safe_path(&workspace.join("missing/../../outside.txt")));
    }

    #[tokio::test]
    async fn test_write_tool_workspace_path_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("work");
        fs::create_dir_all(workspace.join("existing"))
            .await
            .unwrap();
        let tool = WriteTool::with_working_directory(&workspace);
        let canonical_workspace = workspace.canonicalize().unwrap();

        for path in ["new/nested/file.txt", "existing/../file.txt"] {
            let resolved = tool.resolve_workspace_path(path).unwrap();
            let expected = if path.starts_with("new/") {
                canonical_workspace.join(path)
            } else {
                canonical_workspace.join("file.txt")
            };
            assert_eq!(resolved, expected);
            assert!(tool.is_safe_path(&tool.resolve_path(path)));
            assert!(tool.write_file(path, "inside").await.unwrap().success);
            assert_eq!(fs::read_to_string(resolved).await.unwrap(), "inside");
        }

        for path in [
            workspace.join("missing/../denied.txt"),
            temp_dir.path().join("work-other/new.txt"),
            workspace.join("../outside.txt"),
        ] {
            assert!(!tool.is_safe_path(&path));
            assert!(matches!(
                tool.write_file(path.to_str().unwrap(), "denied").await,
                Err(ToolError::PermissionDenied(_))
            ));
            assert!(!path.exists());
        }
        assert!(!workspace.join("missing").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_write_tool_workspace_symlink_paths() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("work");
        let outside = temp_dir.path().join("outside");
        fs::create_dir_all(workspace.join("inside")).await.unwrap();
        fs::create_dir(&outside).await.unwrap();
        symlink(workspace.join("inside"), workspace.join("safe-link")).unwrap();
        symlink(&outside, workspace.join("outside-link")).unwrap();
        symlink(outside.join("missing.txt"), workspace.join("dangling-link")).unwrap();
        let tool = WriteTool::with_working_directory(&workspace);

        let safe_path = "safe-link/new/file.txt";
        let expected = workspace
            .canonicalize()
            .unwrap()
            .join("inside/new/file.txt");
        assert_eq!(tool.resolve_workspace_path(safe_path).unwrap(), expected);
        assert!(tool.write_file(safe_path, "inside").await.unwrap().success);
        assert_eq!(fs::read_to_string(expected).await.unwrap(), "inside");

        for path in ["outside-link/new/file.txt", "dangling-link"] {
            assert!(!tool.is_safe_path(&tool.resolve_path(path)));
            assert!(matches!(
                tool.write_file(path, "denied").await,
                Err(ToolError::PermissionDenied(_))
            ));
        }
        assert!(!outside.join("new").exists());
        assert!(!outside.join("missing.txt").exists());
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
        assert!(tool.mark_file_as_read(file_path.clone()).await.is_ok());

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
