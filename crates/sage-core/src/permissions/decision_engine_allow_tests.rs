use super::*;
use crate::permissions::{
    FilesystemPermissionProfile, PermissionBehavior, PermissionProfileSource,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn supplied_filesystem_keys_can_match_structured_absolute_allow_alias() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(workspace.join("src"))?;
    let allow_pattern = format!("Read({}/src/**)", workspace.to_string_lossy());
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![workspace.to_string_lossy().to_string()],
            ..Default::default()
        },
        allow: vec![PermissionRule::new(
            allow_pattern.clone(),
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };

    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Filesystem,
            "Read",
            vec!["Read(src/lib.rs)".to_string()],
        )
        .with_path(workspace.join("src/lib.rs").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Allow);
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some(allow_pattern.as_str())
    );
    Ok(())
}

#[test]
fn unrelated_supplied_keys_do_not_allow_current_structured_path() {
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec!["/workspace/repo".to_string()],
            ..Default::default()
        },
        default_behavior: PermissionBehavior::Deny,
        default_behavior_set: true,
        default_behavior_source: Some(PermissionProfileSource::Project),
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
            vec![
                "Write(src/lib.rs)".to_string(),
                "Write(secrets/key.txt)".to_string(),
            ],
        )
        .with_path("secrets/key.txt")
        .with_working_directory("/workspace/repo"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
}
