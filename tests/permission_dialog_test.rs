//! Permission dialog integration test
//!
//! This test verifies that dangerous commands are blocked and require confirmation.

use sage_tools::tools::process::requires_user_confirmation;

#[test]
fn test_requires_user_confirmation_rm() {
    // Test rm command detection
    assert!(requires_user_confirmation("rm file.txt").is_some());
    assert!(requires_user_confirmation("rm -rf ./build").is_some());
    assert!(requires_user_confirmation("rm -r /tmp/test").is_some());
}

#[test]
fn test_requires_user_confirmation_rmdir() {
    // Test rmdir command detection
    assert!(requires_user_confirmation("rmdir empty_dir").is_some());
}

#[test]
fn test_requires_user_confirmation_git_force_push() {
    // Test git push --force detection
    assert!(requires_user_confirmation("git push --force").is_some());
    assert!(requires_user_confirmation("git push -f origin main").is_some());
}

#[test]
fn test_requires_user_confirmation_git_reset_hard() {
    // Test git reset --hard detection
    assert!(requires_user_confirmation("git reset --hard HEAD~1").is_some());
}

#[test]
fn test_requires_user_confirmation_drop_database() {
    // Test DROP DATABASE detection
    assert!(requires_user_confirmation("psql -c 'DROP DATABASE test'").is_some());
    assert!(requires_user_confirmation("mysql -e 'DROP TABLE users'").is_some());
}

#[test]
fn test_requires_user_confirmation_docker() {
    // Test docker rm/prune detection
    assert!(requires_user_confirmation("docker rm container_id").is_some());
    assert!(requires_user_confirmation("docker system prune -a").is_some());
}

#[test]
fn test_safe_commands_no_confirmation() {
    // These safe commands should NOT require confirmation
    assert!(requires_user_confirmation("ls -la").is_none());
    assert!(requires_user_confirmation("pwd").is_none());
    assert!(requires_user_confirmation("cat file.txt").is_none());
    assert!(requires_user_confirmation("echo hello").is_none());
    assert!(requires_user_confirmation("git status").is_none());
    assert!(requires_user_confirmation("git push origin main").is_none()); // Normal push is safe
    assert!(requires_user_confirmation("cargo build").is_none());
    assert!(requires_user_confirmation("npm install").is_none());
}

#[test]
fn test_confirmation_message_content() {
    // Test that the confirmation message contains useful information
    let msg = requires_user_confirmation("rm -rf ./build").unwrap();
    assert!(msg.contains("rm -rf ./build"));
    assert!(msg.contains("delete") || msg.contains("recursively"));
}
