//! Tests for sandbox configuration.

use super::*;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_default_config() {
    let config = SandboxConfig::default();
    assert!(config.enabled);
    assert_eq!(config.mode, SandboxMode::Restricted);
    assert!(config.allow_network);
}

#[test]
fn test_permissive_config() {
    let config = SandboxConfig::permissive();
    assert_eq!(config.mode, SandboxMode::Permissive);
    assert!(config.timeout > Duration::from_secs(60));
}

#[test]
fn test_strict_config() {
    let config = SandboxConfig::strict(PathBuf::from("/tmp/sandbox"));
    assert_eq!(config.mode, SandboxMode::Strict);
    assert!(!config.allow_network);
    assert_eq!(config.timeout, Duration::from_secs(30));
}

#[test]
fn test_command_allowed() {
    let config = SandboxConfig::default();

    // Allowed commands
    assert!(config.is_command_allowed("ls"));
    assert!(config.is_command_allowed("git"));
    assert!(config.is_command_allowed("cargo"));

    // Blocked commands
    assert!(!config.is_command_allowed("rm"));
    assert!(!config.is_command_allowed("sudo"));
    assert!(!config.is_command_allowed("kill"));
}

#[test]
fn test_command_with_path() {
    let config = SandboxConfig::default();

    // Commands with full path
    assert!(config.is_command_allowed("/usr/bin/ls"));
    assert!(!config.is_command_allowed("/bin/rm"));
}

#[test]
fn test_host_allowed() {
    let mut config = SandboxConfig::default();

    // All hosts allowed by default
    assert!(config.is_host_allowed("example.com"));

    // Block a host
    config.blocked_hosts.push("blocked.com".to_string());
    assert!(!config.is_host_allowed("blocked.com"));
    assert!(config.is_host_allowed("example.com"));

    // Disable network
    config.allow_network = false;
    assert!(!config.is_host_allowed("example.com"));
}

#[test]
fn test_permissive_allows_more_commands() {
    let config = SandboxConfig::permissive();

    // In permissive mode, most commands are allowed
    assert!(config.is_command_allowed("ls"));
    assert!(config.is_command_allowed("grep"));

    // But always-blocked are still blocked
    assert!(!config.is_command_allowed("sudo"));
}
