//! Extension package manifest validation.

use super::{
    EXTENSION_MANIFEST_SCHEMA_VERSION, ExtensionPackageManifest, PackageAssetKind,
    PackageAssetPath, PackageAssets, PackageDependency, PackageFileAsset, PackageHookAsset,
    PackageManifestError, PackageMcpServerAsset,
};
use std::collections::BTreeSet;
use std::path::{Component, Path};

impl ExtensionPackageManifest {
    /// Parse and validate a TOML extension manifest.
    pub fn from_toml_str(contents: &str) -> Result<Self, PackageManifestError> {
        let manifest: Self =
            toml::from_str(contents).map_err(|err| PackageManifestError::Parse(err.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate this manifest.
    pub fn validate(&self) -> Result<(), PackageManifestError> {
        if self.schema_version != EXTENSION_MANIFEST_SCHEMA_VERSION {
            return Err(PackageManifestError::UnsupportedSchema {
                found: self.schema_version,
                expected: EXTENSION_MANIFEST_SCHEMA_VERSION,
            });
        }

        validate_package_id(&self.id)?;
        validate_non_empty(&self.id, "name", &self.name)?;
        validate_non_empty(&self.id, "version", &self.version)?;

        let mut declared_permissions = BTreeSet::new();
        for permission in &self.permissions {
            validate_non_empty(&self.id, "permissions", permission)?;
            declared_permissions.insert(permission);
        }

        for dependency in &self.dependencies {
            dependency.validate(&self.id)?;
        }

        self.assets.validate(&self.id, &declared_permissions)?;
        Ok(())
    }

    /// Return all package-relative asset paths declared by this manifest.
    pub fn asset_paths(&self) -> Vec<PackageAssetPath> {
        self.assets.asset_paths()
    }
}

impl PackageAssets {
    fn validate(
        &self,
        package_id: &str,
        declared_permissions: &BTreeSet<&String>,
    ) -> Result<(), PackageManifestError> {
        validate_file_assets(
            package_id,
            PackageAssetKind::Skill,
            &self.skills,
            declared_permissions,
        )?;
        validate_mcp_assets(package_id, &self.mcp_servers, declared_permissions)?;
        validate_hook_assets(package_id, &self.hooks, declared_permissions)?;
        validate_file_assets(
            package_id,
            PackageAssetKind::Command,
            &self.commands,
            declared_permissions,
        )
    }

    fn asset_paths(&self) -> Vec<PackageAssetPath> {
        let mut paths = Vec::new();
        paths.extend(
            self.skills
                .iter()
                .map(|asset| asset.asset_path(PackageAssetKind::Skill)),
        );
        paths.extend(
            self.commands
                .iter()
                .map(|asset| asset.asset_path(PackageAssetKind::Command)),
        );
        paths.extend(
            self.hooks
                .iter()
                .map(|asset| asset.asset_path(PackageAssetKind::Hook)),
        );
        paths.extend(self.mcp_servers.iter().filter_map(|asset| {
            asset
                .path
                .as_ref()
                .map(|path| PackageAssetPath::new(PackageAssetKind::McpServer, &asset.id, path))
        }));
        paths
    }
}

impl PackageFileAsset {
    fn asset_path(&self, kind: PackageAssetKind) -> PackageAssetPath {
        PackageAssetPath::new(kind, &self.id, &self.path)
    }
}

impl PackageHookAsset {
    fn asset_path(&self, kind: PackageAssetKind) -> PackageAssetPath {
        PackageAssetPath::new(kind, &self.id, &self.path)
    }
}

impl PackageDependency {
    fn validate(&self, package_id: &str) -> Result<(), PackageManifestError> {
        validate_identifier(package_id, &self.id, "dependency.id")?;
        validate_not_dot_only(package_id, "dependency.id", &self.id)?;
        validate_non_empty(package_id, "dependency.version", &self.version)
    }
}

impl PackageAssetPath {
    fn new(kind: PackageAssetKind, asset_id: &str, path: &Path) -> Self {
        Self {
            kind,
            asset_id: asset_id.to_string(),
            path: path.to_path_buf(),
        }
    }
}

fn validate_file_assets(
    package_id: &str,
    kind: PackageAssetKind,
    assets: &[PackageFileAsset],
    declared_permissions: &BTreeSet<&String>,
) -> Result<(), PackageManifestError> {
    let mut ids = BTreeSet::new();
    for asset in assets {
        validate_asset_id(package_id, kind, &asset.id)?;
        validate_relative_asset_path(package_id, &asset.id, &asset.path)?;
        validate_required_permissions(
            package_id,
            &asset.id,
            &asset.required_permissions,
            declared_permissions,
        )?;
        if !ids.insert(asset.id.as_str()) {
            return Err(PackageManifestError::DuplicateAssetId {
                package_id: package_id.to_string(),
                kind,
                asset_id: asset.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_hook_assets(
    package_id: &str,
    assets: &[PackageHookAsset],
    declared_permissions: &BTreeSet<&String>,
) -> Result<(), PackageManifestError> {
    let mut ids = BTreeSet::new();
    for asset in assets {
        validate_asset_id(package_id, PackageAssetKind::Hook, &asset.id)?;
        validate_relative_asset_path(package_id, &asset.id, &asset.path)?;
        if let Some(event) = &asset.event {
            validate_hook_event(package_id, event)?;
        }
        validate_required_permissions(
            package_id,
            &asset.id,
            &asset.required_permissions,
            declared_permissions,
        )?;
        if !ids.insert(asset.id.as_str()) {
            return Err(PackageManifestError::DuplicateAssetId {
                package_id: package_id.to_string(),
                kind: PackageAssetKind::Hook,
                asset_id: asset.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_mcp_assets(
    package_id: &str,
    assets: &[PackageMcpServerAsset],
    declared_permissions: &BTreeSet<&String>,
) -> Result<(), PackageManifestError> {
    let mut ids = BTreeSet::new();
    for asset in assets {
        validate_asset_id(package_id, PackageAssetKind::McpServer, &asset.id)?;
        if let Some(path) = &asset.path {
            validate_relative_asset_path(package_id, &asset.id, path)?;
        } else {
            validate_inline_mcp_asset(package_id, asset)?;
        }
        validate_required_permissions(
            package_id,
            &asset.id,
            &asset.required_permissions,
            declared_permissions,
        )?;
        if !ids.insert(asset.id.as_str()) {
            return Err(PackageManifestError::DuplicateAssetId {
                package_id: package_id.to_string(),
                kind: PackageAssetKind::McpServer,
                asset_id: asset.id.clone(),
            });
        }
    }
    Ok(())
}

/// Validate a package id before it is used in manifest or store APIs.
pub fn validate_package_id(package_id: &str) -> Result<(), PackageManifestError> {
    validate_identifier(package_id, package_id, "id")?;
    validate_not_dot_only(package_id, "id", package_id)
}

fn validate_asset_id(
    package_id: &str,
    kind: PackageAssetKind,
    value: &str,
) -> Result<(), PackageManifestError> {
    validate_identifier(package_id, value, "asset.id").map_err(|_| {
        PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field: "asset.id",
            reason: format!("{} asset id must be non-empty and identifier-safe", kind),
        }
    })
}

fn validate_not_dot_only(
    package_id: &str,
    field: &'static str,
    value: &str,
) -> Result<(), PackageManifestError> {
    if value == "." || value == ".." || !value.chars().any(|ch| ch.is_ascii_alphanumeric()) {
        Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field,
            reason: "value must include an ASCII letter or number and cannot be . or .."
                .to_string(),
        })
    } else {
        Ok(())
    }
}

fn validate_inline_mcp_asset(
    package_id: &str,
    asset: &PackageMcpServerAsset,
) -> Result<(), PackageManifestError> {
    match asset.transport.as_deref() {
        Some("stdio") => validate_non_empty(
            package_id,
            "assets.mcp_servers.command",
            asset.command.as_deref().unwrap_or_default(),
        ),
        Some("http") | Some("websocket") => validate_non_empty(
            package_id,
            "assets.mcp_servers.url",
            asset.url.as_deref().unwrap_or_default(),
        ),
        Some(other) if !other.trim().is_empty() => Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field: "assets.mcp_servers.transport",
            reason: format!("unsupported MCP transport '{other}'"),
        }),
        _ => Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field: "assets.mcp_servers.transport",
            reason: "inline MCP server declarations require transport".to_string(),
        }),
    }
}

fn validate_identifier(
    package_id: &str,
    value: &str,
    field: &'static str,
) -> Result<(), PackageManifestError> {
    validate_non_empty(package_id, field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        Ok(())
    } else {
        Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field,
            reason: "value may only contain ASCII letters, numbers, dot, hyphen, and underscore"
                .to_string(),
        })
    }
}

fn validate_non_empty(
    package_id: &str,
    field: &'static str,
    value: &str,
) -> Result<(), PackageManifestError> {
    if value.trim().is_empty() {
        Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field,
            reason: "value cannot be empty".to_string(),
        })
    } else {
        Ok(())
    }
}

/// Validate a package-relative path without touching the filesystem.
pub fn validate_relative_asset_path(
    package_id: &str,
    asset_id: &str,
    path: &Path,
) -> Result<(), PackageManifestError> {
    if path.as_os_str().is_empty() {
        return Err(PackageManifestError::InvalidPath {
            package_id: package_id.to_string(),
            asset_id: asset_id.to_string(),
            path: path.display().to_string(),
            reason: "path cannot be empty".to_string(),
        });
    }

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PackageManifestError::InvalidPath {
                    package_id: package_id.to_string(),
                    asset_id: asset_id.to_string(),
                    path: path.display().to_string(),
                    reason: "path must stay inside the package root".to_string(),
                });
            }
            Component::CurDir => {
                return Err(PackageManifestError::InvalidPath {
                    package_id: package_id.to_string(),
                    asset_id: asset_id.to_string(),
                    path: path.display().to_string(),
                    reason: "path must not contain current-directory segments".to_string(),
                });
            }
        }
    }

    Ok(())
}

fn validate_required_permissions(
    package_id: &str,
    asset_id: &str,
    permissions: &[String],
    declared_permissions: &BTreeSet<&String>,
) -> Result<(), PackageManifestError> {
    if permissions.is_empty() {
        return Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field: "assets.required_permissions",
            reason: format!("asset '{asset_id}' must explicitly declare required permissions"),
        });
    }

    for permission in permissions {
        if !declared_permissions.contains(permission) {
            return Err(PackageManifestError::UndeclaredPermission {
                package_id: package_id.to_string(),
                asset_id: asset_id.to_string(),
                permission: permission.clone(),
            });
        }
    }
    Ok(())
}

fn validate_hook_event(package_id: &str, event: &str) -> Result<(), PackageManifestError> {
    let valid = matches!(
        event,
        "pre_tool_use"
            | "post_tool_use"
            | "post_tool_use_failure"
            | "user_prompt_submit"
            | "session_start"
            | "session_end"
            | "subagent_start"
            | "subagent_stop"
            | "permission_request"
            | "pre_compact"
            | "notification"
            | "stop"
            | "status_line"
            | "PreToolUse"
            | "PostToolUse"
            | "PostToolUseFailure"
            | "UserPromptSubmit"
            | "SessionStart"
            | "SessionEnd"
            | "SubagentStart"
            | "SubagentStop"
            | "PermissionRequest"
            | "PreCompact"
            | "Notification"
            | "Stop"
            | "StatusLine"
    );
    if valid {
        Ok(())
    } else {
        Err(PackageManifestError::InvalidField {
            package_id: package_id.to_string(),
            field: "assets.hooks.event",
            reason: format!("unknown hook event '{event}'"),
        })
    }
}
