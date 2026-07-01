use super::*;
use crate::permissions::{
    FilesystemPermissionProfile, NetworkPermissionProfile, PermissionProfileSource,
};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use tempfile::TempDir;

#[test]
fn deny_rules_take_precedence_over_allow_rules() {
    let profile = PermissionProfile::default()
        .add_allow("Bash(*)", PermissionProfileSource::User)
        .add_deny("Bash(rm *)", PermissionProfileSource::Project);
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Exec,
        "Bash",
        vec!["Bash(rm -rf target)".to_string()],
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("matched deny rule"));
}

#[test]
fn workspace_path_is_allowed_when_rule_matches() {
    let workspace = std::env::current_dir().unwrap().join("workspace");
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![workspace.to_string_lossy().to_string()],
            ..Default::default()
        },
        allow: vec![PermissionRule::new(
            "Write(src/**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Filesystem,
            "Write",
            vec!["Write(src/main.rs)".to_string()],
        )
        .with_path(workspace.join("src/main.rs").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Allow);
}

#[test]
fn outside_workspace_path_is_denied() {
    let workspace = std::env::current_dir().unwrap().join("workspace");
    let outside = std::env::current_dir().unwrap().join("outside/file.txt");
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![workspace.to_string_lossy().to_string()],
            ..Default::default()
        },
        allow: vec![PermissionRule::new(
            "Write(**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Filesystem,
            "Write",
            vec!["Write(outside/file.txt)".to_string()],
        )
        .with_path(outside.to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("outside configured workspace"));
}

#[cfg(unix)]
#[test]
fn symlink_escape_path_is_denied_as_outside_workspace() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let outside = temp_dir.path().join("outside");
    fs::create_dir_all(&workspace)?;
    fs::create_dir_all(&outside)?;
    std::os::unix::fs::symlink(&outside, workspace.join("outside_link"))?;

    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![workspace.to_string_lossy().to_string()],
            ..Default::default()
        },
        allow: vec![PermissionRule::new(
            "Write(**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Filesystem,
            "Write",
            vec!["Write(outside_link/file.txt)".to_string()],
        )
        .with_path(workspace.join("outside_link/file.txt").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("outside configured workspace"));

    Ok(())
}

#[test]
fn protected_workspace_path_is_denied_before_allow() {
    let workspace = std::env::current_dir().unwrap().join("workspace");
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![workspace.to_string_lossy().to_string()],
            ..Default::default()
        },
        allow: vec![PermissionRule::new(
            "Write(**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Filesystem,
            "Write",
            vec!["Write(.git/config)".to_string()],
        )
        .with_path(workspace.join(".git/config").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("protected"));
}

#[test]
fn network_disabled_denies_network_action() {
    let profile = PermissionProfile {
        network: NetworkPermissionProfile { enabled: false },
        allow: vec![PermissionRule::new(
            "WebFetch(https://example.com/**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Network,
            "WebFetch",
            vec!["WebFetch(https://example.com/docs)".to_string()],
        )
        .with_network_target("https://example.com/docs"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("network access is disabled"));
}

#[test]
fn unsupported_requested_sandbox_fails_closed() {
    let profile = PermissionProfile::default();
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Sandbox,
            "Bash",
            vec!["Bash(cargo test)".to_string()],
        )
        .with_required_sandbox(SandboxSupport::Unsupported),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Unsupported);
}
