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

#[test]
fn deserialized_filesystem_fragment_defaults_missing_fields() -> serde_json::Result<()> {
    let project: PermissionProfile =
        serde_json::from_str(r#"{"source":"project","filesystem":{"workspace_roots":["/repo"]}}"#)?;

    assert_eq!(project.filesystem.workspace_roots, vec!["/repo"]);
    assert!(!project.filesystem.allow_outside_workspace);
    assert_eq!(
        project.filesystem.protected_paths,
        vec![".git", ".sage", ".ssh"]
    );
    assert_eq!(
        project.domain_sources.filesystem,
        Some(PermissionProfileSource::Project)
    );
    Ok(())
}

#[test]
fn deserialized_rules_default_missing_source_to_profile_source() -> serde_json::Result<()> {
    let project: PermissionProfile = serde_json::from_str(
        r#"{"source":"project","allow":[{"pattern":"Bash(cargo *)"}],"deny":[{"pattern":"Read(secrets/**)"}]}"#,
    )?;

    assert_eq!(project.allow[0].source, PermissionProfileSource::Project);
    assert_eq!(project.deny[0].source, PermissionProfileSource::Project);
    Ok(())
}

#[test]
fn deserialized_rule_source_cannot_exceed_profile_source() -> serde_json::Result<()> {
    let project: PermissionProfile = serde_json::from_str(
        r#"{
            "source":"project",
            "allow":[{"pattern":"Bash(cargo *)","source":"runtime"}]
        }"#,
    )?;

    assert_eq!(project.allow[0].source, PermissionProfileSource::Project);
    Ok(())
}

#[test]
fn deserializing_profile_without_source_is_rejected() {
    let result = serde_json::from_str::<PermissionProfile>(r#"{"network":{"enabled":true}}"#);

    assert!(result.is_err());
}

#[test]
fn serialized_rules_only_profile_preserves_unset_provenance() -> serde_json::Result<()> {
    let project = PermissionProfile::default()
        .with_source(PermissionProfileSource::Project)
        .add_allow("Bash(cargo *)", PermissionProfileSource::Project);
    let serialized = serde_json::to_value(&project)?;

    assert!(serialized.get("source").is_some());
    assert!(serialized.get("allow").is_some());
    assert!(serialized.get("filesystem").is_none());
    assert!(serialized.get("network").is_none());
    assert!(serialized.get("exec").is_none());
    assert!(serialized.get("sandbox").is_none());
    assert!(serialized.get("default_behavior").is_none());

    let round_tripped: PermissionProfile = serde_json::from_value(serialized)?;
    assert!(round_tripped.domain_sources.is_empty());
    assert!(!round_tripped.default_behavior_set);
    assert_eq!(round_tripped.default_behavior_source, None);
    Ok(())
}

#[test]
fn deserialized_domain_source_cannot_exceed_profile_source() -> serde_json::Result<()> {
    let project: PermissionProfile = serde_json::from_str(
        r#"{
            "source":"project",
            "network":{"enabled":true},
            "domain_sources":{"network":"runtime"}
        }"#,
    )?;
    let mut runtime = PermissionProfile::default()
        .with_source(PermissionProfileSource::Runtime)
        .with_network_profile(
            NetworkPermissionProfile { enabled: false },
            PermissionProfileSource::Runtime,
        );

    runtime.merge(project);

    assert!(!runtime.network.enabled);
    assert_eq!(
        runtime.domain_sources.network,
        Some(PermissionProfileSource::Runtime)
    );
    Ok(())
}

#[test]
fn deserialized_default_behavior_source_cannot_exceed_profile_source() -> serde_json::Result<()> {
    let project: PermissionProfile = serde_json::from_str(
        r#"{
            "source":"project",
            "default_behavior":"allow",
            "default_behavior_source":"runtime"
        }"#,
    )?;
    let mut runtime = PermissionProfile::default()
        .with_source(PermissionProfileSource::Runtime)
        .with_default_behavior(PermissionBehavior::Deny);

    runtime.merge(project);

    assert_eq!(runtime.default_behavior, PermissionBehavior::Deny);
    assert_eq!(
        runtime.default_behavior_source,
        Some(PermissionProfileSource::Runtime)
    );
    Ok(())
}

#[test]
fn deserialized_explicit_ask_default_tracks_source() -> serde_json::Result<()> {
    let local: PermissionProfile =
        serde_json::from_str(r#"{"source":"local","default_behavior":"ask"}"#)?;
    let mut user = PermissionProfile::default()
        .with_source(PermissionProfileSource::User)
        .with_default_behavior(PermissionBehavior::Allow);

    user.merge(local);

    assert_eq!(user.default_behavior, PermissionBehavior::Ask);
    assert!(user.default_behavior_set);
    assert_eq!(
        user.default_behavior_source,
        Some(PermissionProfileSource::Local)
    );
    Ok(())
}

#[test]
fn with_source_reassigns_default_behavior_source() {
    let mut local = PermissionProfile::default()
        .with_source(PermissionProfileSource::Local)
        .with_default_behavior(PermissionBehavior::Deny);
    let user = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .with_source(PermissionProfileSource::User);

    local.merge(user);

    assert_eq!(local.default_behavior, PermissionBehavior::Deny);
    assert_eq!(
        local.default_behavior_source,
        Some(PermissionProfileSource::Local)
    );
}

#[test]
fn managed_merge_does_not_loosen_existing_deny_default() {
    let managed = PermissionProfile::default()
        .with_source(PermissionProfileSource::Managed)
        .with_default_behavior(PermissionBehavior::Ask);
    let mut profile = PermissionProfile::default()
        .with_source(PermissionProfileSource::Project)
        .with_default_behavior(PermissionBehavior::Deny);

    profile.merge(managed);

    assert_eq!(profile.default_behavior, PermissionBehavior::Deny);
}

#[test]
fn managed_merge_preserves_workspace_roots_when_adding_protected_paths() {
    let managed_filesystem = FilesystemPermissionProfile {
        protected_paths: vec!["secrets/**".to_string()],
        ..Default::default()
    };
    let managed = PermissionProfile::default()
        .with_source(PermissionProfileSource::Managed)
        .with_filesystem_profile(managed_filesystem, PermissionProfileSource::Managed);
    let mut profile = PermissionProfile::default().with_filesystem_profile(
        FilesystemPermissionProfile {
            workspace_roots: vec!["/workspace".to_string()],
            allow_outside_workspace: false,
            protected_paths: vec![".env".to_string()],
        },
        PermissionProfileSource::Project,
    );

    profile.merge(managed);

    assert_eq!(profile.filesystem.workspace_roots, vec!["/workspace"]);
    assert!(
        profile
            .filesystem
            .protected_paths
            .contains(&".env".to_string())
    );
    assert!(
        profile
            .filesystem
            .protected_paths
            .contains(&"secrets/**".to_string())
    );
}
