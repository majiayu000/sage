//! Extension package manifest schema.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

#[path = "package_manifest_validation.rs"]
mod validation;
pub use validation::validate_package_id;
pub use validation::validate_relative_asset_path;

/// Current extension package manifest schema version.
pub const EXTENSION_MANIFEST_SCHEMA_VERSION: u32 = 0;

/// Extension package manifest file name.
pub const EXTENSION_MANIFEST_FILE: &str = "sage-extension.toml";

/// Error returned for invalid extension package manifests.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackageManifestError {
    /// The manifest could not be parsed.
    #[error("failed to parse extension manifest: {0}")]
    Parse(String),

    /// The manifest declares an unsupported schema version.
    #[error("unsupported extension manifest schema_version {found}; expected {expected}")]
    UnsupportedSchema { found: u32, expected: u32 },

    /// A required field is missing or empty.
    #[error("extension package '{package_id}' has invalid field '{field}': {reason}")]
    InvalidField {
        package_id: String,
        field: &'static str,
        reason: String,
    },

    /// A package-relative path is invalid.
    #[error(
        "extension package '{package_id}' asset '{asset_id}' has invalid path '{path}': {reason}"
    )]
    InvalidPath {
        package_id: String,
        asset_id: String,
        path: String,
        reason: String,
    },

    /// An asset requires a permission that the package did not declare.
    #[error(
        "extension package '{package_id}' asset '{asset_id}' requires undeclared permission '{permission}'"
    )]
    UndeclaredPermission {
        package_id: String,
        asset_id: String,
        permission: String,
    },

    /// The same asset id was declared more than once for one asset kind.
    #[error("extension package '{package_id}' declares duplicate {kind} asset id '{asset_id}'")]
    DuplicateAssetId {
        package_id: String,
        kind: PackageAssetKind,
        asset_id: String,
    },
}

/// Extension asset kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageAssetKind {
    /// Skill asset.
    Skill,
    /// MCP server asset.
    McpServer,
    /// Hook asset.
    Hook,
    /// Slash command asset.
    Command,
}

impl fmt::Display for PackageAssetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Skill => write!(f, "skill"),
            Self::McpServer => write!(f, "mcp_server"),
            Self::Hook => write!(f, "hook"),
            Self::Command => write!(f, "command"),
        }
    }
}

/// Versioned extension package manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPackageManifest {
    /// Manifest schema version. GH-86 introduces version 0.
    pub schema_version: u32,
    /// Stable package id.
    pub id: String,
    /// Human-readable package name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Declared package assets.
    #[serde(default)]
    pub assets: PackageAssets,
    /// Package dependencies that must be installed before enable.
    #[serde(default)]
    pub dependencies: Vec<PackageDependency>,
    /// Package-level permissions accepted for this package.
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Additional package metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}

/// Assets declared by an extension package.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageAssets {
    /// Skill assets.
    #[serde(default)]
    pub skills: Vec<PackageFileAsset>,
    /// MCP server assets.
    #[serde(default)]
    pub mcp_servers: Vec<PackageMcpServerAsset>,
    /// Hook assets.
    #[serde(default)]
    pub hooks: Vec<PackageHookAsset>,
    /// Slash command assets.
    #[serde(default)]
    pub commands: Vec<PackageFileAsset>,
}

/// File-backed package asset declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageFileAsset {
    /// Asset id inside its asset kind.
    pub id: String,
    /// Package-relative asset path.
    pub path: PathBuf,
    /// Permissions this asset needs from the package permission list.
    #[serde(default)]
    pub required_permissions: Vec<String>,
    /// Additional asset metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}

/// Hook package asset declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageHookAsset {
    /// Hook id.
    pub id: String,
    /// Package-relative hook configuration path.
    pub path: PathBuf,
    /// Optional runtime hook event.
    #[serde(default)]
    pub event: Option<String>,
    /// Permissions this hook needs from the package permission list.
    #[serde(default)]
    pub required_permissions: Vec<String>,
    /// Additional hook metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}

/// MCP server package asset declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMcpServerAsset {
    /// MCP server id.
    pub id: String,
    /// Optional package-relative MCP config path.
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Inline MCP transport.
    #[serde(default)]
    pub transport: Option<String>,
    /// Inline stdio command.
    #[serde(default)]
    pub command: Option<String>,
    /// Inline stdio args.
    #[serde(default)]
    pub args: Vec<String>,
    /// Inline stdio env.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Inline HTTP/WebSocket URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Inline HTTP headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Optional request timeout.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Permissions this MCP server needs from the package permission list.
    #[serde(default)]
    pub required_permissions: Vec<String>,
    /// Additional MCP metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}

/// Extension package dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageDependency {
    /// Dependency package id.
    pub id: String,
    /// Required version constraint. GH-86 stores this for later resolution.
    pub version: String,
    /// Whether the dependency is optional.
    #[serde(default)]
    pub optional: bool,
}

/// Package-relative asset path plus identifying metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageAssetPath {
    /// Asset kind.
    pub kind: PackageAssetKind,
    /// Asset id.
    pub asset_id: String,
    /// Package-relative asset path.
    pub path: PathBuf,
}

#[cfg(test)]
#[path = "package_manifest_tests.rs"]
mod tests;
