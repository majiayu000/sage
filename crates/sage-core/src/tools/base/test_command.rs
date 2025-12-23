//! CommandTool tests

#![cfg(test)]

use crate::tools::base::command_tool::CommandTool;
use super::test_mocks::MockCommandTool;

#[test]
fn test_command_tool_no_restrictions() {
    let temp_dir = std::env::temp_dir();
    let tool = MockCommandTool::new(vec![], temp_dir);

    // With empty allowed list, all commands should be allowed
    assert!(tool.is_command_allowed("ls"));
    assert!(tool.is_command_allowed("echo hello"));
    assert!(tool.is_command_allowed("rm -rf /"));
}

#[test]
fn test_command_tool_with_restrictions() {
    let temp_dir = std::env::temp_dir();
    let tool = MockCommandTool::new(
        vec!["ls".to_string(), "cat".to_string(), "echo".to_string()],
        temp_dir,
    );

    // Allowed commands
    assert!(tool.is_command_allowed("ls"));
    assert!(tool.is_command_allowed("ls -la"));
    assert!(tool.is_command_allowed("cat file.txt"));
    assert!(tool.is_command_allowed("echo hello"));

    // Disallowed commands
    assert!(!tool.is_command_allowed("rm file.txt"));
    assert!(!tool.is_command_allowed("sudo su"));
    assert!(!tool.is_command_allowed("wget malicious.com"));
}

#[test]
fn test_command_tool_prefix_matching() {
    let temp_dir = std::env::temp_dir();
    let tool = MockCommandTool::new(vec!["git".to_string()], temp_dir);

    // All git commands should be allowed
    assert!(tool.is_command_allowed("git status"));
    assert!(tool.is_command_allowed("git commit -m 'test'"));
    assert!(tool.is_command_allowed("git push origin main"));

    // Non-git commands should be disallowed
    assert!(!tool.is_command_allowed("ls"));
    assert!(!tool.is_command_allowed("github"));
}
