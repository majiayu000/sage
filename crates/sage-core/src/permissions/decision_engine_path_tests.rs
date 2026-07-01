use super::*;
#[cfg(windows)]
use crate::permissions::PermissionBehavior;
use crate::permissions::{FilesystemPermissionProfile, PermissionProfileSource};
use std::fs;
use tempfile::TempDir;

#[test]
fn relative_workspace_root_uses_request_working_directory() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(workspace.join("src"))?;
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![".".to_string()],
            ..Default::default()
        },
        allow: vec![PermissionRule::new(
            "Write(src/**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };

    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Filesystem, "Write", Vec::new())
            .with_path("src/main.rs")
            .with_working_directory(workspace.to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Allow);
    assert_eq!(decision.audit_key, "Write(src/main.rs)");
    Ok(())
}

#[cfg(windows)]
#[test]
fn protected_workspace_path_matches_case_insensitively_on_windows() {
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![r"C:\repo".to_string()],
            ..Default::default()
        },
        default_behavior: PermissionBehavior::Allow,
        default_behavior_set: true,
        default_behavior_source: Some(PermissionProfileSource::Project),
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
            vec![r"Write(C:\repo\.GIT\config)".to_string()],
        )
        .with_path(r"C:\repo\.GIT\config"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("protected"));
}
