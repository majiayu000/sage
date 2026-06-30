use super::*;

const VALID_FIXTURE: &str = r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
description = "Review helpers"
permissions = ["skills:read", "commands:run", "hooks:run", "network:mcp"]

[[dependencies]]
id = "acme.base"
version = "1.0.0"
optional = true

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
id = "doc-search"
transport = "stdio"
command = "node"
args = ["server.js"]
required_permissions = ["network:mcp"]
"#;

#[test]
fn package_manifest_fixture_parses() {
    let manifest = ExtensionPackageManifest::from_toml_str(VALID_FIXTURE).unwrap();

    assert_eq!(manifest.schema_version, EXTENSION_MANIFEST_SCHEMA_VERSION);
    assert_eq!(manifest.id, "acme.review");
    assert_eq!(manifest.assets.skills.len(), 1);
    assert_eq!(manifest.assets.commands.len(), 1);
    assert_eq!(manifest.assets.hooks.len(), 1);
    assert_eq!(manifest.assets.mcp_servers.len(), 1);
    assert_eq!(manifest.asset_paths().len(), 3);
}

#[test]
fn unknown_manifest_fields_fail_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
unknown = true
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::Parse(_)));
}

#[test]
fn path_escape_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"

[[assets.commands]]
id = "review"
path = "../review.md"
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::InvalidPath { .. }));
}

#[test]
fn undeclared_asset_permission_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["commands:run"]

[[assets.commands]]
id = "review"
path = "commands/review.md"
required_permissions = ["network:mcp"]
"#,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        PackageManifestError::UndeclaredPermission { .. }
    ));
}

#[test]
fn duplicate_asset_id_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["commands:run"]

[[assets.commands]]
id = "review"
path = "commands/review.md"
required_permissions = ["commands:run"]

[[assets.commands]]
id = "review"
path = "commands/review2.md"
required_permissions = ["commands:run"]
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::DuplicateAssetId { .. }));
}

#[test]
fn missing_asset_required_permissions_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["commands:run"]

[[assets.commands]]
id = "review"
path = "commands/review.md"
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::InvalidField { .. }));
}

#[test]
fn unknown_hook_event_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"

[[assets.hooks]]
id = "preflight"
path = "hooks/preflight.toml"
event = "surprise"
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::InvalidField { .. }));
}

#[test]
fn dot_only_package_id_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = ".."
name = "Acme Review"
version = "1.0.0"
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::InvalidField { .. }));
}

#[test]
fn unsupported_inline_mcp_transport_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"

[[assets.mcp_servers]]
id = "docs"
transport = "pipe"
command = "node"
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::InvalidField { .. }));
}

#[test]
fn inline_mcp_missing_required_field_fails_closed() {
    let err = ExtensionPackageManifest::from_toml_str(
        r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"

[[assets.mcp_servers]]
id = "docs"
transport = "stdio"
"#,
    )
    .unwrap_err();

    assert!(matches!(err, PackageManifestError::InvalidField { .. }));
}
