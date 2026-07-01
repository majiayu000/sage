use super::*;
use crate::permissions::{
    FilesystemPermissionProfile, NetworkPermissionProfile, PermissionBehavior,
    PermissionProfileSource, SandboxPermissionProfile,
};
use std::fs;
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
fn allow_decision_records_matched_rule_for_audit() {
    let profile = PermissionProfile {
        allow: vec![
            PermissionRule::new("Read(docs/**)", PermissionProfileSource::User),
            PermissionRule::new("Read(src/**)", PermissionProfileSource::Project),
        ],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Tool,
        "Read",
        vec!["Read(src/lib.rs)".to_string()],
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Allow);
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("Read(src/**)")
    );
}

#[test]
fn filesystem_decision_without_path_fails_closed() {
    let profile =
        PermissionProfile::default().add_allow("Write(**)", PermissionProfileSource::Project);
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Filesystem,
        "Write",
        vec!["Write(src/lib.rs)".to_string()],
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("require a request path"));
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

#[test]
fn path_request_without_workspace_roots_fails_closed() -> std::io::Result<()> {
    let path = std::env::current_dir()?.join("outside/file.txt");
    let profile =
        PermissionProfile::default().add_allow("Write(**)", PermissionProfileSource::Project);
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Filesystem,
            "Write",
            vec!["Write(outside/file.txt)".to_string()],
        )
        .with_path(path.to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(
        decision
            .reason
            .contains("no workspace roots are configured")
    );

    Ok(())
}

#[test]
fn relative_path_uses_request_working_directory() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let nested = workspace.join("nested");
    let outside = temp_dir.path().join("outside");
    fs::create_dir_all(&nested)?;
    fs::create_dir_all(&outside)?;

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
            vec!["Write(../../outside/file.txt)".to_string()],
        )
        .with_path("../../outside/file.txt")
        .with_working_directory(nested.to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("outside configured workspace"));

    Ok(())
}

#[test]
fn absolute_protected_path_is_denied_before_allow() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let protected = temp_dir.path().join("protected");
    fs::create_dir_all(&protected)?;

    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            protected_paths: vec![protected.to_string_lossy().to_string()],
            allow_outside_workspace: true,
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
            vec!["Write(secret.txt)".to_string()],
        )
        .with_path(protected.join("secret.txt").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("protected"));

    Ok(())
}

#[cfg(windows)]
#[test]
fn windows_absolute_protected_path_is_denied_before_allow() {
    let protected = r"C:\Users\me\.ssh";
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            protected_paths: vec![protected.to_string()],
            allow_outside_workspace: true,
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
            vec![r"Write(C:\Users\me\.ssh\id_ed25519)".to_string()],
        )
        .with_path(r"C:\Users\me\.ssh\id_ed25519"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("protected"));
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

#[cfg(unix)]
#[test]
fn symlinked_protected_path_is_denied() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let protected_target = workspace.join("metadata");
    fs::create_dir_all(&workspace)?;
    fs::create_dir_all(&protected_target)?;
    std::os::unix::fs::symlink(&protected_target, workspace.join(".sage"))?;

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
            vec!["Write(.sage/config.json)".to_string()],
        )
        .with_path(workspace.join(".sage/config.json").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("protected"));

    Ok(())
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
fn network_target_matches_rules_when_permission_keys_are_empty() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(https://internal.example/**)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Network, "WebFetch", Vec::new())
            .with_network_target("https://internal.example/private"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("WebFetch(https://internal.example/**)")
    );
}

#[test]
fn filesystem_path_matches_rules_when_permission_keys_are_empty() {
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            allow_outside_workspace: true,
            ..Default::default()
        },
        default_behavior: PermissionBehavior::Allow,
        default_behavior_set: true,
        default_behavior_source: Some(PermissionProfileSource::Project),
        deny: vec![PermissionRule::new(
            "Write(/tmp/secret/**)",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Filesystem, "Write", Vec::new())
            .with_path("/tmp/secret/file.txt"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("Write(/tmp/secret/**)")
    );
}

#[test]
fn filesystem_path_fallback_uses_workspace_relative_rule_keys() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(workspace.join("src"))?;
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
        PermissionDecisionInput::new(PermissionAction::Filesystem, "Write", Vec::new())
            .with_path(workspace.join("src/main.rs").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Allow);
    assert_eq!(decision.audit_key, "Write(src/main.rs)");
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("Write(src/**)")
    );
    Ok(())
}

#[test]
fn filesystem_path_fallback_preserves_absolute_workspace_rule_keys() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let secret = workspace.join("secret");
    fs::create_dir_all(&secret)?;
    fs::create_dir_all(workspace.join("public"))?;
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            workspace_roots: vec![workspace.to_string_lossy().to_string()],
            ..Default::default()
        },
        default_behavior: PermissionBehavior::Allow,
        default_behavior_set: true,
        default_behavior_source: Some(PermissionProfileSource::Project),
        deny: vec![PermissionRule::new(
            format!("Write({}/**)", secret.to_string_lossy()),
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Filesystem, "Write", Vec::new())
            .with_path(workspace.join("public/../secret/key").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    let expected_rule = format!("Write({}/**)", secret.to_string_lossy());
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some(expected_rule.as_str())
    );
    Ok(())
}

#[test]
fn bare_tool_rule_matches_when_permission_keys_are_empty() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny("Bash", PermissionProfileSource::Project);
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Tool,
        "Bash",
        Vec::new(),
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(decision.audit_key, "Bash");
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("Bash")
    );
}

#[test]
fn bare_exec_rule_matches_when_permission_keys_are_empty() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny("Bash", PermissionProfileSource::Project);
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Exec,
        "Bash",
        Vec::new(),
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(decision.audit_key, "Bash");
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("Bash")
    );
}

#[test]
fn bare_sandbox_rule_matches_when_permission_keys_are_empty() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny("Bash", PermissionProfileSource::Project);
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Sandbox, "Bash", Vec::new())
            .with_required_sandbox(SandboxSupport::Supported),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(decision.audit_key, "Bash");
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("Bash")
    );
}

#[test]
fn outside_filesystem_path_fallback_normalizes_traversal_before_matching() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let public = temp_dir.path().join("public");
    let secret = temp_dir.path().join("secret");
    fs::create_dir_all(&public)?;
    fs::create_dir_all(&secret)?;
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            allow_outside_workspace: true,
            ..Default::default()
        },
        default_behavior: PermissionBehavior::Allow,
        default_behavior_set: true,
        default_behavior_source: Some(PermissionProfileSource::Project),
        deny: vec![PermissionRule::new(
            format!("Write({}/**)", secret.to_string_lossy()),
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Filesystem, "Write", Vec::new())
            .with_path(public.join("../secret/key").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    Ok(())
}

#[cfg(unix)]
#[test]
fn outside_filesystem_path_fallback_matches_canonical_symlink_target() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let secret = temp_dir.path().join("secret");
    let link = temp_dir.path().join("secret_link");
    fs::create_dir_all(&secret)?;
    std::os::unix::fs::symlink(&secret, &link)?;
    let profile = PermissionProfile {
        filesystem: FilesystemPermissionProfile {
            allow_outside_workspace: true,
            ..Default::default()
        },
        default_behavior: PermissionBehavior::Allow,
        default_behavior_set: true,
        default_behavior_source: Some(PermissionProfileSource::Project),
        deny: vec![PermissionRule::new(
            format!("Write({}/**)", secret.to_string_lossy()),
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Filesystem, "Write", Vec::new())
            .with_path(link.join("key").to_string_lossy()),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    Ok(())
}

#[test]
fn network_target_fallback_normalizes_url_before_matching() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(https://internal.example/**)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Network, "WebFetch", Vec::new())
            .with_network_target("HTTPS://INTERNAL.EXAMPLE:443/private#token"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision.audit_key,
        "WebFetch(https://internal.example/private)"
    );
}

#[test]
fn network_target_fallback_strips_url_credentials_before_matching() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(https://internal.example/**)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Network, "WebFetch", Vec::new())
            .with_network_target("https://user:password@internal.example/private"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision.audit_key,
        "WebFetch(https://internal.example/private)"
    );
}

#[test]
fn network_target_fallback_matches_exact_origin_without_trailing_slash() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(https://internal.example)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Network, "WebFetch", Vec::new())
            .with_network_target("HTTPS://INTERNAL.EXAMPLE:443#fragment"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(decision.audit_key, "WebFetch(https://internal.example)");
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

#[test]
fn preflight_denial_preserves_matched_rule_source() {
    let profile = PermissionProfile::default();
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Tool, "Grep", vec!["Grep".to_string()])
            .with_preflight_denies(vec![
                PermissionPreflight::new(
                    "Grep search overlaps deny rule 'Grep(secrets/**)'",
                    Some("Grep(secrets/**)".to_string()),
                )
                .with_source(PermissionProfileSource::Managed),
            ]),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision.matched_rule.as_ref().map(|rule| rule.source),
        Some(PermissionProfileSource::Managed)
    );
}

#[test]
fn required_profile_sandbox_with_unknown_support_fails_closed() {
    let profile = PermissionProfile {
        sandbox: SandboxPermissionProfile { required: true },
        allow: vec![PermissionRule::new(
            "Bash",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Sandbox,
        "Bash",
        Vec::new(),
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Unsupported);
    assert_eq!(decision.audit_key, "Bash");
}

#[test]
fn deserialized_missing_sandbox_support_fails_closed_when_profile_requires_sandbox()
-> serde_json::Result<()> {
    let profile = PermissionProfile {
        sandbox: SandboxPermissionProfile { required: true },
        allow: vec![PermissionRule::new(
            "Bash",
            PermissionProfileSource::Project,
        )],
        ..Default::default()
    };
    let input: PermissionDecisionInput = serde_json::from_value(serde_json::json!({
        "action": "sandbox",
        "tool_name": "Bash",
        "permission_keys": [],
        "path": null,
        "network_target": null,
        "requires_sandbox": false,
        "preflight_denies": [],
        "scoped_allows": []
    }))?;

    let decision = PermissionDecisionEngine::new(profile).decide(input);

    assert_eq!(decision.kind, PermissionDecisionKind::Unsupported);
    assert_eq!(decision.audit_key, "Bash");
    Ok(())
}
