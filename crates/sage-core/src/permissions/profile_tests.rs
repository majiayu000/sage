use super::*;

#[test]
fn merge_keeps_rules_and_higher_precedence_domains() {
    let mut base = PermissionProfile::default()
        .with_source(PermissionProfileSource::User)
        .with_network_profile(
            NetworkPermissionProfile { enabled: false },
            PermissionProfileSource::User,
        )
        .add_allow("Read(src/**)", PermissionProfileSource::User);

    let local = PermissionProfile::default()
        .with_source(PermissionProfileSource::Local)
        .with_network_profile(
            NetworkPermissionProfile { enabled: true },
            PermissionProfileSource::Local,
        )
        .add_deny("Read(secrets/**)", PermissionProfileSource::Local)
        .with_default_behavior(PermissionBehavior::Deny);

    base.merge(local);

    assert_eq!(base.allow.len(), 1);
    assert_eq!(base.deny.len(), 1);
    assert!(base.network.enabled);
    assert_eq!(base.default_behavior, PermissionBehavior::Deny);
    assert!(base.default_behavior_set);
}

#[test]
fn lower_precedence_profile_cannot_downgrade_domains() {
    let mut runtime = PermissionProfile::default()
        .with_source(PermissionProfileSource::Runtime)
        .with_exec_profile(
            ExecPermissionProfile { enabled: false },
            PermissionProfileSource::Runtime,
        );

    let user = PermissionProfile::default()
        .with_source(PermissionProfileSource::User)
        .with_exec_profile(
            ExecPermissionProfile { enabled: true },
            PermissionProfileSource::User,
        );

    runtime.merge(user);

    assert!(!runtime.exec.enabled);
}

#[test]
fn settings_fragment_does_not_override_domains_or_higher_default() {
    let mut runtime = PermissionProfile::default()
        .with_source(PermissionProfileSource::Runtime)
        .with_filesystem_profile(
            FilesystemPermissionProfile {
                workspace_roots: vec!["/repo".to_string()],
                ..Default::default()
            },
            PermissionProfileSource::Runtime,
        )
        .with_network_profile(
            NetworkPermissionProfile { enabled: false },
            PermissionProfileSource::Runtime,
        )
        .with_default_behavior(PermissionBehavior::Deny);
    let settings = PermissionSettings {
        allow: vec!["Bash(echo *)".to_string()],
        default_behavior: SettingsPermissionBehavior::Allow,
        default_behavior_set: true,
        ..Default::default()
    };

    runtime.merge(PermissionProfile::from_settings(&settings));

    assert_eq!(runtime.filesystem.workspace_roots, vec!["/repo"]);
    assert!(!runtime.network.enabled);
    assert_eq!(runtime.default_behavior, PermissionBehavior::Deny);
    assert_eq!(runtime.allow.len(), 1);
}

#[test]
fn deserialized_fragment_tracks_present_domain_sources() -> serde_json::Result<()> {
    let user: PermissionProfile =
        serde_json::from_str(r#"{"source":"user","network":{"enabled":false}}"#)?;
    let mut merged = PermissionProfile::default();

    merged.merge(user);

    assert!(!merged.network.enabled);
    assert_eq!(
        merged.domain_sources.network,
        Some(PermissionProfileSource::User)
    );
    Ok(())
}
