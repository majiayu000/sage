//! CommandTool tests

#![cfg(test)]

use super::test_mocks::MockCommandTool;
use crate::tools::base::command_tool::CommandTool;

#[test]
fn test_command_tool_no_restrictions() {
    let temp_dir = std::env::temp_dir();
    let tool = MockCommandTool::new(vec![], temp_dir);

    // With empty allowed list, all commands should be allowed
    assert!(tool.is_command_allowed(&["ls".into()]));
    assert!(tool.is_command_allowed(&["echo".into(), "hello".into()]));
    assert!(tool.is_command_allowed(&["rm".into(), "-rf".into(), "/".into()]));
}

#[test]
fn test_command_tool_with_restrictions() {
    let temp_dir = std::env::temp_dir();
    let tool = MockCommandTool::new(
        vec!["ls".to_string(), "cat".to_string(), "echo".to_string()],
        temp_dir,
    );

    // Allowed commands
    assert!(tool.is_command_allowed(&["ls".into()]));
    assert!(tool.is_command_allowed(&["ls".into(), "-la".into()]));
    assert!(tool.is_command_allowed(&["cat".into(), "file.txt".into()]));
    assert!(tool.is_command_allowed(&["echo".into(), "hello".into()]));

    // Disallowed commands
    assert!(!tool.is_command_allowed(&["rm".into(), "file.txt".into()]));
    assert!(!tool.is_command_allowed(&["sudo".into(), "su".into()]));
    assert!(!tool.is_command_allowed(&["wget".into(), "malicious.com".into()]));
}

#[test]
fn test_command_tool_prefix_matching() {
    let temp_dir = std::env::temp_dir();
    let tool = MockCommandTool::new(vec!["git".to_string()], temp_dir);

    // All git commands should be allowed
    assert!(tool.is_command_allowed(&["git".into(), "status".into()]));
    assert!(tool.is_command_allowed(&["git".into(), "commit".into(), "-m".into(), "test".into()]));
    assert!(tool.is_command_allowed(&[
        "git".into(),
        "push".into(),
        "origin".into(),
        "main".into()
    ]));

    // Non-git commands should be disallowed
    assert!(!tool.is_command_allowed(&["ls".into()]));
    assert!(!tool.is_command_allowed(&["github".into()]));
}
