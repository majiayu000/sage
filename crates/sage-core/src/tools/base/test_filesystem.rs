//! FileSystemTool tests

#![cfg(test)]

use super::test_mocks::MockTool;
use crate::tools::base::filesystem_tool::FileSystemTool;

#[test]
fn test_filesystem_tool_resolve_absolute_path() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir.clone());

    let absolute = temp_dir.join("test.txt");
    let resolved = tool.resolve_path(&absolute.to_string_lossy());
    assert_eq!(resolved, absolute);
}

#[test]
fn test_filesystem_tool_resolve_relative_path() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir.clone());

    let resolved = tool.resolve_path("test.txt");
    assert_eq!(resolved, temp_dir.join("test.txt"));
}

#[test]
fn test_filesystem_tool_is_safe_path_within_working_dir() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir.clone());

    // Create a test file within the temp directory
    let safe_path = temp_dir.join("safe_file.txt");
    assert!(tool.is_safe_path(&safe_path));
}

#[test]
fn test_filesystem_tool_is_safe_path_traversal_attack() {
    let temp_dir = std::env::temp_dir();
    let tool = MockTool::new(temp_dir.clone());

    // Try to escape using parent directory
    let unsafe_path = temp_dir.join("../../../etc/passwd");
    // After canonicalization, this should be outside the working directory
    // Note: This test may behave differently on different systems
    // The key is that is_safe_path should prevent escaping the working directory
    let canonical_unsafe = unsafe_path.canonicalize();
    if let Ok(canon) = canonical_unsafe {
        // Only test if canonicalization succeeds
        if !canon.starts_with(&temp_dir) {
            assert!(!tool.is_safe_path(&unsafe_path));
        }
    }
}
