use super::*;
use crate::commands::{CommandRegistry, CommandSource, SlashCommand};
use crate::hooks::HookRegistry;
use crate::plugins::package_store::InstalledPackageState;
use crate::skills::SkillRegistry;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn package_record(root: &Path) -> InstalledPackageRecord {
    fs::create_dir_all(root.join("skills/reviewer")).unwrap();
    fs::create_dir_all(root.join("commands")).unwrap();
    fs::create_dir_all(root.join("hooks")).unwrap();
    fs::write(
        root.join("skills/reviewer/SKILL.md"),
        r#"---
description: Package reviewer
when_to_use: When package review is needed
allowed_tools: [Read, Grep]
user_invocable: true
---
review prompt
"#,
    )
    .unwrap();
    fs::write(
        root.join("commands/review.md"),
        "---\ndescription: Review command\n---\nreview prompt",
    )
    .unwrap();
    fs::write(
        root.join("hooks/preflight.toml"),
        r#"
pattern = "*"
name = "placeholder"
hook_type = "pre_tool_execution"
type = "command"
command = "echo ok"
"#,
    )
    .unwrap();

    let manifest = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["skills:read", "commands:run", "hooks:run", "network:mcp"]

[[assets.skills]]
id = "reviewer"
path = "skills/reviewer/SKILL.md"
required_permissions = ["skills:read"]

[[assets.commands]]
id = "review"
path = "commands/review.md"
required_permissions = ["commands:run"]

[[assets.hooks]]
id = "preflight"
path = "hooks/preflight.toml"
event = "pre_tool_use"
required_permissions = ["hooks:run"]

[[assets.mcp_servers]]
id = "docs"
transport = "stdio"
command = "node"
args = ["server.js"]
required_permissions = ["network:mcp"]
"#,
    )
    .unwrap();

    InstalledPackageRecord {
        package_id: manifest.id.clone(),
        version: manifest.version.clone(),
        install_root: root.to_path_buf(),
        state: InstalledPackageState::Disabled,
        manifest,
    }
}

#[test]
fn package_registry_bridge_enable_disable_is_reversible() {
    let temp = TempDir::new().unwrap();
    let record = package_record(temp.path());
    let mut skills = SkillRegistry::new(temp.path());
    let mut commands = CommandRegistry::new(temp.path());
    let hooks = HookRegistry::new();
    let mut bridge = PackageRegistryBridge::new();

    let registered = bridge
        .enable_package(&record, &mut skills, &mut commands, &hooks)
        .unwrap();

    assert_eq!(registered.skills, vec!["reviewer"]);
    assert!(skills.contains("reviewer"));
    let skill = skills.get("reviewer").unwrap();
    assert_eq!(
        skill.when_to_use.as_deref(),
        Some("When package review is needed")
    );
    assert!(skill.user_invocable());
    assert!(commands.contains("review"));
    assert!(hooks.contains_hook_name("preflight"));
    assert!(bridge.mcp_server("docs").is_some());

    let removed = bridge
        .disable_package("acme.review", &mut skills, &mut commands, &hooks)
        .unwrap();

    assert_eq!(removed.hooks, vec!["preflight"]);
    assert!(!skills.contains("reviewer"));
    assert!(!commands.contains("review"));
    assert!(!hooks.contains_hook_name("preflight"));
    assert!(bridge.mcp_server("docs").is_none());
}

#[test]
fn package_registry_conflict_fails_before_mutation() {
    let temp = TempDir::new().unwrap();
    let record = package_record(temp.path());
    let mut skills = SkillRegistry::new(temp.path());
    let mut commands = CommandRegistry::new(temp.path());
    commands.register(
        SlashCommand::new("review", "existing"),
        CommandSource::Project,
    );
    let hooks = HookRegistry::new();
    let mut bridge = PackageRegistryBridge::new();

    let err = bridge
        .enable_package(&record, &mut skills, &mut commands, &hooks)
        .unwrap_err();

    assert!(matches!(err, PackageError::RegistryConflict { .. }));
    assert!(!skills.contains("reviewer"));
    assert!(bridge.mcp_servers().is_empty());
}

#[test]
fn path_backed_mcp_config_fails_closed_for_invalid_transport() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("mcp")).unwrap();
    fs::write(
        temp.path().join("mcp/docs.toml"),
        r#"
transport = "pipe"
command = "node"
"#,
    )
    .unwrap();

    let manifest = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["network:mcp"]

[[assets.mcp_servers]]
id = "docs"
path = "mcp/docs.toml"
required_permissions = ["network:mcp"]
"#,
    )
    .unwrap();
    let record = InstalledPackageRecord {
        package_id: manifest.id.clone(),
        version: manifest.version.clone(),
        install_root: temp.path().to_path_buf(),
        state: InstalledPackageState::Disabled,
        manifest,
    };
    let mut skills = SkillRegistry::new(temp.path());
    let mut commands = CommandRegistry::new(temp.path());
    let hooks = HookRegistry::new();
    let mut bridge = PackageRegistryBridge::new();

    let err = bridge
        .enable_package(&record, &mut skills, &mut commands, &hooks)
        .unwrap_err();

    assert!(matches!(err, PackageError::Registry { .. }));
    assert!(bridge.mcp_servers().is_empty());
}

#[test]
fn path_backed_mcp_config_fails_closed_for_missing_required_field() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("mcp")).unwrap();
    fs::write(
        temp.path().join("mcp/docs.toml"),
        r#"
transport = "stdio"
"#,
    )
    .unwrap();

    let manifest = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["network:mcp"]

[[assets.mcp_servers]]
id = "docs"
path = "mcp/docs.toml"
required_permissions = ["network:mcp"]
"#,
    )
    .unwrap();
    let record = InstalledPackageRecord {
        package_id: manifest.id.clone(),
        version: manifest.version.clone(),
        install_root: temp.path().to_path_buf(),
        state: InstalledPackageState::Disabled,
        manifest,
    };
    let mut skills = SkillRegistry::new(temp.path());
    let mut commands = CommandRegistry::new(temp.path());
    let hooks = HookRegistry::new();
    let mut bridge = PackageRegistryBridge::new();

    let err = bridge
        .enable_package(&record, &mut skills, &mut commands, &hooks)
        .unwrap_err();

    assert!(matches!(err, PackageError::Registry { .. }));
    assert!(bridge.mcp_servers().is_empty());
}
