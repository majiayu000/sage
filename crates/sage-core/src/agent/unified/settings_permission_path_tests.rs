use super::settings_permission_test_support::{
    glob_call, grep_call_without_path, notebook_call, path_call, read_call, workspace_dir,
};
use super::*;
use crate::settings::types::PermissionSettings;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_settings_permission_matches_workspace_relative_absolute_path() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Read(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("/workspace/sage/src/lib.rs"),
        workspace_dir(),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_keeps_outside_absolute_paths_distinct() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Read(tmp/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("/tmp/secret.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_normalizes_windows_separators() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("src\\secret.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_grep_and_glob_paths() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec![
                "Grep(secrets/**)".to_string(),
                "Glob(secrets/**)".to_string(),
            ],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let grep_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "/workspace/sage/secrets/private.txt"),
        workspace_dir(),
    );
    let glob_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("secrets/**", None),
        workspace_dir(),
    );

    assert!(matches!(
        grep_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        glob_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_denies_broad_glob_overlapping_scoped_deny() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Glob(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let recursive_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("**/*", None),
        workspace_dir(),
    );
    let root_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("*", None),
        workspace_dir(),
    );

    assert!(matches!(
        recursive_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        root_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_denies_glob_search_path_overlapping_scoped_deny() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Glob(src/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let broad_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("**/*", Some("/workspace/sage/src")),
        workspace_dir(),
    );
    let narrow_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("*.rs", Some("/workspace/sage/src")),
        workspace_dir(),
    );

    assert!(matches!(
        broad_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert_eq!(narrow_decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_denies_recursive_grep_overlapping_scoped_deny() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Grep(src/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "src"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_denies_workspace_wide_grep_scope() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Grep(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &grep_call_without_path("token"),
        workspace_dir(),
    );
    let dot_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "."),
        workspace_dir(),
    );
    let root_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "/workspace/sage"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        dot_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        root_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_glob_path_joined_with_pattern() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Glob(src/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("secrets/**", Some("/workspace/sage/src")),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_absolute_path_with_relative_working_dir() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Read(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };
    let cwd = std::env::current_dir().expect("current dir available");
    let absolute_path = cwd.join("src/lib.rs").to_string_lossy().to_string();

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call(&absolute_path),
        Path::new("."),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_normalizes_relative_path_components() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        settings_permission_paths::workspace_relative_path(
            "src/../secrets/key.txt",
            workspace_dir(),
        ),
        "secrets/key.txt"
    );

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("src/../secrets/key.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[cfg(unix)]
#[test]
fn test_settings_permission_canonicalizes_existing_symlink_target() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("secrets"))?;
    fs::write(temp_dir.path().join("secrets/key.txt"), "secret")?;
    std::os::unix::fs::symlink(
        temp_dir.path().join("secrets"),
        temp_dir.path().join("public"),
    )?;

    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("public/key.txt"),
        temp_dir.path(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_settings_permission_resolves_symlink_before_parent_components() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("secrets"))?;
    fs::create_dir(temp_dir.path().join("secrets/subdir"))?;
    fs::write(temp_dir.path().join("secrets/key.txt"), "secret")?;
    std::os::unix::fs::symlink(
        temp_dir.path().join("secrets/subdir"),
        temp_dir.path().join("allowed"),
    )?;

    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("allowed/../key.txt"),
        temp_dir.path(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    Ok(())
}

#[test]
fn test_settings_permission_matches_notebook_path() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["NotebookEdit(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &notebook_call("/workspace/sage/secrets/private.ipynb"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}
