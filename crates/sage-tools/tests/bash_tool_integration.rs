//! Integration tests for the Bash tool

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::tools::process::BashTool;
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
async fn test_bash_simple_commands() {
    let tool = BashTool::new();

    println!("\n=== Test 1: Simple echo command ===");
    let call = create_tool_call(
        "test-1",
        "bash",
        json!({
            "argv": ["echo", "Hello, World!"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.as_ref().unwrap().contains("Hello, World!"));
    println!("✓ Echo command executed successfully");

    println!("\n=== Test 2: pwd command ===");
    let call = create_tool_call(
        "test-2",
        "bash",
        json!({
            "argv": ["pwd"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.is_some());
    println!("✓ pwd command executed successfully");

    println!("\n=== Test 3: date command ===");
    let call = create_tool_call(
        "test-3",
        "bash",
        json!({
            "argv": ["date", "+%Y"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        // Should contain a year (4 digits)
        assert!(output.chars().filter(|c| c.is_numeric()).count() >= 4);
    }
    println!("✓ date command executed successfully");
}

#[tokio::test]
async fn test_bash_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 4: Create file with echo ===");
    let call = create_tool_call(
        "test-4",
        "bash",
        json!({
            "argv": ["sh", "-c", "echo 'Test content' > test.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    // Verify file was created
    let file_path = temp_dir.path().join("test.txt");
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("Test content"));
    println!("✓ File created successfully");

    println!("\n=== Test 5: Read file with cat ===");
    let call = create_tool_call(
        "test-5",
        "bash",
        json!({
            "argv": ["cat", "test.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.as_ref().unwrap().contains("Test content"));
    println!("✓ File read successfully");

    println!("\n=== Test 6: List files with ls ===");
    let call = create_tool_call(
        "test-6",
        "bash",
        json!({
            "argv": ["ls", "-la"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.as_ref().unwrap().contains("test.txt"));
    println!("✓ Directory listing successful");
}

#[tokio::test]
async fn test_bash_pipe_operations() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    // Create test file with trailing newline for accurate wc -l count
    let file_path = temp_dir.path().join("numbers.txt");
    fs::write(&file_path, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n")
        .await
        .unwrap();

    println!("\n=== Test 7: Pipe with head ===");
    let call = create_tool_call(
        "test-7",
        "bash",
        json!({
            "argv": ["sh", "-c", "cat numbers.txt | head -n 3"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("1"));
        assert!(output.contains("2"));
        assert!(output.contains("3"));
        assert!(!output.contains("10"));
    }
    println!("✓ Pipe with head executed successfully");

    println!("\n=== Test 8: Pipe with tail ===");
    let call = create_tool_call(
        "test-8",
        "bash",
        json!({
            "argv": ["sh", "-c", "cat numbers.txt | tail -n 2"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("9"));
        assert!(output.contains("10"));
        assert!(!output.contains("1\n"));
    }
    println!("✓ Pipe with tail executed successfully");

    println!("\n=== Test 9: Pipe with wc ===");
    let call = create_tool_call(
        "test-9",
        "bash",
        json!({
            "argv": ["sh", "-c", "cat numbers.txt | wc -l"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        // wc -l output may have leading/trailing whitespace
        let trimmed = output.trim();
        assert!(trimmed == "10" || trimmed.ends_with("10"));
    }
    println!("✓ Pipe with wc executed successfully");
}

#[tokio::test]
async fn test_bash_grep_operations() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    // Create test file
    let file_path = temp_dir.path().join("data.txt");
    fs::write(&file_path, "apple\nbanana\ncherry\napricot\navocado")
        .await
        .unwrap();

    println!("\n=== Test 10: grep pattern matching ===");
    let call = create_tool_call(
        "test-10",
        "bash",
        json!({
            "argv": ["grep", "^a", "data.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("apple"));
        assert!(output.contains("apricot"));
        assert!(output.contains("avocado"));
        assert!(!output.contains("banana"));
        assert!(!output.contains("cherry"));
    }
    println!("✓ grep pattern matching successful");

    println!("\n=== Test 11: grep with count ===");
    let call = create_tool_call(
        "test-11",
        "bash",
        json!({
            "argv": ["grep", "-c", "^a", "data.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("3"));
    }
    println!("✓ grep count successful");
}

#[tokio::test]
async fn test_bash_find_operations() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    // Create some test files
    fs::write(temp_dir.path().join("file1.txt"), "content")
        .await
        .unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content")
        .await
        .unwrap();
    fs::write(temp_dir.path().join("script.sh"), "#!/bin/bash")
        .await
        .unwrap();

    println!("\n=== Test 12: find by name pattern ===");
    let call = create_tool_call(
        "test-12",
        "bash",
        json!({
            "argv": ["sh", "-c", "find . -name '*.txt' | head -5"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
        assert!(!output.contains("script.sh"));
    }
    println!("✓ find by name pattern successful");
}

#[tokio::test]
async fn test_bash_working_directory() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).await.unwrap();

    let tool = BashTool::with_working_directory(&subdir);

    println!("\n=== Test 13: Command runs in working directory ===");
    let call = create_tool_call(
        "test-13",
        "bash",
        json!({
            "argv": ["pwd"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        // The output should contain the subdirectory path
        assert!(output.contains("subdir"));
    }
    println!("✓ Working directory set correctly");
}

#[tokio::test]
async fn test_bash_error_handling() {
    let tool = BashTool::new();

    println!("\n=== Test 14: Invalid command ===");
    let call = create_tool_call(
        "test-14",
        "bash",
        json!({
            "argv": ["nonexistent_command_xyz123"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
    println!("✓ Invalid command error handled correctly");

    println!("\n=== Test 15: Command with non-zero exit code ===");
    let call = create_tool_call(
        "test-15",
        "bash",
        json!({
            "argv": ["ls", "/nonexistent_directory_xyz"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(!result.success);
    assert!(result.exit_code.is_some());
    println!("✓ Non-zero exit code handled correctly");
}

#[tokio::test]
async fn test_bash_empty_command() {
    let tool = BashTool::new();

    println!("\n=== Test 16: Empty command ===");
    let call = create_tool_call(
        "test-16",
        "bash",
        json!({
            "argv": []
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    println!("✓ Empty command rejected");

    println!("\n=== Test 17: Whitespace-only command ===");
    let call = create_tool_call(
        "test-17",
        "bash",
        json!({
            "argv": [""]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    println!("✓ Whitespace-only command rejected");
}

#[tokio::test]
async fn test_bash_missing_command() {
    let tool = BashTool::new();

    println!("\n=== Test 18: Missing command parameter ===");
    let call = create_tool_call("test-18", "bash", json!({}));

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Missing") || err.to_string().contains("argv"));
    println!("✓ Missing command parameter detected");
}

#[tokio::test]
async fn test_bash_metadata() {
    let tool = BashTool::new();

    println!("\n=== Test 19: Result metadata ===");
    let call = create_tool_call(
        "test-19",
        "bash",
        json!({
            "argv": ["echo", "test"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    // Check metadata
    assert!(result.metadata.contains_key("argv"));
    assert!(result.metadata.contains_key("working_directory"));
    assert!(result.execution_time_ms.is_some());
    println!("✓ Metadata populated correctly");
}

#[tokio::test]
async fn test_bash_text_processing() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    // Create test file
    let file_path = temp_dir.path().join("text.txt");
    fs::write(&file_path, "one two three\nfour five six\nseven eight nine")
        .await
        .unwrap();

    println!("\n=== Test 20: awk text processing ===");
    let call = create_tool_call(
        "test-20",
        "bash",
        json!({
            "argv": ["awk", "{print $2}", "text.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("two"));
        assert!(output.contains("five"));
        assert!(output.contains("eight"));
    }
    println!("✓ awk text processing successful");

    println!("\n=== Test 21: sed text replacement ===");
    let call = create_tool_call(
        "test-21",
        "bash",
        json!({
            "argv": ["sed", "s/two/TWO/", "text.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("one TWO three"));
    }
    println!("✓ sed text replacement successful");
}

#[tokio::test]
async fn test_bash_environment_variables() {
    let tool = BashTool::new();

    println!("\n=== Test 22: Access PATH variable ===");
    let call = create_tool_call(
        "test-22",
        "bash",
        json!({
            "argv": ["sh", "-c", "echo $PATH | head -c 20"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.is_some());
    println!("✓ PATH environment variable accessible");

    println!("\n=== Test 23: Access HOME variable ===");
    let call = create_tool_call(
        "test-23",
        "bash",
        json!({
            "argv": ["sh", "-c", "echo $HOME | head -c 50"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.output.is_some());
    println!("✓ HOME environment variable accessible");
}

#[tokio::test]
async fn test_bash_multiple_commands_piped() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 24: Complex pipe chain ===");

    // Create test data
    fs::write(
        temp_dir.path().join("data.txt"),
        "apple 10\nbanana 5\ncherry 15\napricot 8",
    )
    .await
    .unwrap();

    let call = create_tool_call(
        "test-24",
        "bash",
        json!({
            "argv": ["sh", "-c", "cat data.txt | grep '^a' | wc -l"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    if let Some(output) = &result.output {
        assert!(output.contains("2"));
    }
    println!("✓ Complex pipe chain executed successfully");
}

#[tokio::test]
async fn test_bash_redirection() {
    let temp_dir = TempDir::new().unwrap();
    let tool = BashTool::with_working_directory(temp_dir.path());

    println!("\n=== Test 25: Output redirection ===");
    let call = create_tool_call(
        "test-25",
        "bash",
        json!({
            "argv": ["sh", "-c", "echo 'redirected output' > output.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    // Verify file was created with correct content
    let content = fs::read_to_string(temp_dir.path().join("output.txt"))
        .await
        .unwrap();
    assert!(content.contains("redirected output"));
    println!("✓ Output redirection successful");

    println!("\n=== Test 26: Append redirection ===");
    let call = create_tool_call(
        "test-26",
        "bash",
        json!({
            "argv": ["sh", "-c", "echo 'appended line' >> output.txt"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(temp_dir.path().join("output.txt"))
        .await
        .unwrap();
    assert!(content.contains("redirected output"));
    assert!(content.contains("appended line"));
    println!("✓ Append redirection successful");
}

#[tokio::test]
async fn test_bash_allowed_commands() {
    let tool = BashTool::new().with_allowed_commands(vec!["echo".to_string(), "pwd".to_string()]);

    println!("\n=== Test 27: Allowed command (echo) ===");
    let call = create_tool_call(
        "test-27",
        "bash",
        json!({
            "argv": ["echo", "allowed"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    println!("✓ Allowed command executed successfully");

    println!("\n=== Test 28: Disallowed command (ls) ===");
    let call = create_tool_call(
        "test-28",
        "bash",
        json!({
            "argv": ["ls"]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    println!("✓ Disallowed command rejected");
}

#[tokio::test]
async fn test_bash_stdout_stderr() {
    let tool = BashTool::new();

    println!("\n=== Test 29: Command with stderr output ===");
    let call = create_tool_call(
        "test-29",
        "bash",
        json!({
            "argv": ["sh", "-c", "ls /nonexistent 2>&1"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(!result.success);
    // Should capture error message
    assert!(result.error.is_some() || result.output.is_some());
    println!("✓ stderr output captured");
}

#[tokio::test]
async fn test_bash_execution_time() {
    let tool = BashTool::new();

    println!("\n=== Test 30: Execution time tracking ===");
    let call = create_tool_call(
        "test-30",
        "bash",
        json!({
            "argv": ["echo", "test"]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(result.execution_time_ms.is_some());
    println!(
        "✓ Execution time tracked: {} ms",
        result.execution_time_ms.unwrap()
    );
}

#[tokio::test]
async fn test_destructive_command_requires_confirmation() {
    let tool = BashTool::new();

    println!("\n=== Test 31: Destructive commands require confirmation ===");

    // Test rm command without confirmation - should return error
    let call = create_tool_call(
        "test-31a",
        "bash",
        json!({
            "argv": ["rm", "test_file.txt"]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("DESTRUCTIVE COMMAND BLOCKED"));
    println!("✓ rm command blocked without confirmation");

    // Test rmdir command without confirmation
    let call = create_tool_call(
        "test-31b",
        "bash",
        json!({
            "argv": ["rmdir", "empty_dir"]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    println!("✓ rmdir command blocked without confirmation");

    // Test git push --force without confirmation
    let call = create_tool_call(
        "test-31c",
        "bash",
        json!({
            "argv": ["git", "push", "--force", "origin", "main"]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    println!("✓ git push --force blocked without confirmation");
}

#[tokio::test]
async fn test_destructive_command_with_confirmation() {
    let tool = BashTool::new();

    println!("\n=== Test 32: Destructive commands allowed with confirmation ===");

    // Create a temp directory and file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_delete.txt");
    fs::write(&test_file, "test content").await.unwrap();
    assert!(test_file.exists());

    // Test rm command with user_confirmed=true - should execute
    let call = create_tool_call(
        "test-32",
        "bash",
        json!({
            "argv": ["rm", test_file.to_string_lossy()],
            "user_confirmed": true
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    assert!(!test_file.exists());
    println!("✓ rm command executed with user confirmation");
}

#[tokio::test]
async fn test_safe_commands_no_confirmation_needed() {
    let tool = BashTool::new();

    println!("\n=== Test 33: Safe commands need no confirmation ===");

    // These commands should execute without user_confirmed
    let safe_commands: Vec<Vec<String>> = vec![
        vec!["ls".to_string(), "-la".to_string()],
        vec!["pwd".to_string()],
        vec!["echo".to_string(), "hello".to_string()],
        vec!["git".to_string(), "status".to_string()],
        vec!["cargo".to_string(), "--version".to_string()],
    ];

    for (i, argv) in safe_commands.iter().enumerate() {
        let call = create_tool_call(
            &format!("test-33-{}", i),
            "bash",
            json!({
                "argv": argv
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success, "Command '{:?}' should succeed", argv);
        println!("✓ '{:?}' executed without confirmation", argv);
    }
}
