//! Tests for grep tool

#[cfg(test)]
mod grep_tests {
    use crate::tools::file_ops::grep::GrepTool;
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
    async fn test_grep_basic_search() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");

        fs::write(&file1, "Hello World\nThis is a test\nAnother line")
            .await
            .unwrap();
        fs::write(&file2, "No match here\nJust some text")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-1",
            "Grep",
            json!({
                "pattern": "test",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("test1.txt"));
        assert!(!result.output.as_ref().unwrap().contains("test2.txt"));
    }

    #[tokio::test]
    async fn test_grep_content_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.rs");

        fs::write(
            &file,
            "fn main() {\n    println!(\"Hello\");\n}\n\nfn test() {\n    println!(\"Test\");\n}",
        )
        .await
        .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-2",
            "Grep",
            json!({
                "pattern": "fn.*\\(\\)",
                "output_mode": "content",
                "-n": true
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("fn main()"));
        assert!(output.contains("fn test()"));
        assert!(output.contains("1:")); // Line numbers
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");

        fs::write(&file, "Hello World\nhello world\nHELLO WORLD")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-3",
            "Grep",
            json!({
                "pattern": "hello",
                "-i": true,
                "output_mode": "count"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("3")); // Should match all 3 lines
    }

    #[tokio::test]
    async fn test_grep_with_context() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");

        fs::write(&file, "line 1\nline 2\nMATCH\nline 4\nline 5")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-4",
            "Grep",
            json!({
                "pattern": "MATCH",
                "output_mode": "content",
                "-A": 1,
                "-B": 1,
                "-n": true
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("line 2"));
        assert!(output.contains("MATCH"));
        assert!(output.contains("line 4"));
    }

    #[tokio::test]
    async fn test_grep_glob_filter() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("test.rs");
        let txt_file = temp_dir.path().join("test.txt");

        fs::write(&rust_file, "fn main() {}").await.unwrap();
        fs::write(&txt_file, "fn main() {}").await.unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-5",
            "Grep",
            json!({
                "pattern": "fn",
                "glob": "*.rs",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("test.rs"));
        assert!(!output.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_grep_type_filter() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("test.rs");
        let py_file = temp_dir.path().join("test.py");

        fs::write(&rust_file, "fn main() {}").await.unwrap();
        fs::write(&py_file, "def main():").await.unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-6",
            "Grep",
            json!({
                "pattern": "main",
                "type": "rust",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("test.rs"));
        assert!(!output.contains("test.py"));
    }

    #[tokio::test]
    async fn test_grep_head_limit() {
        let temp_dir = TempDir::new().unwrap();

        for i in 1..=5 {
            let file = temp_dir.path().join(format!("test{}.txt", i));
            fs::write(&file, "MATCH").await.unwrap();
        }

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-7",
            "Grep",
            json!({
                "pattern": "MATCH",
                "output_mode": "files_with_matches",
                "head_limit": 3
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        let line_count = output.lines().filter(|l| l.contains("test")).count();
        assert_eq!(line_count, 3); // Should only show 3 files
    }

    #[tokio::test]
    async fn test_grep_invalid_regex() {
        let temp_dir = TempDir::new().unwrap();
        let tool = GrepTool::with_working_directory(temp_dir.path());

        let call = create_tool_call(
            "test-8",
            "Grep",
            json!({
                "pattern": "[invalid(regex",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");

        fs::write(&file, "Some content\nNo matches here")
            .await
            .unwrap();

        let tool = GrepTool::with_working_directory(temp_dir.path());
        let call = create_tool_call(
            "test-9",
            "Grep",
            json!({
                "pattern": "nonexistent",
                "output_mode": "files_with_matches"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("No matches found"));
    }

    #[test]
    fn test_grep_schema() {
        let tool = GrepTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "Grep");
        assert!(!schema.description.is_empty());
    }
}
