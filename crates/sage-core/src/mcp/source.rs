//! MCP source declarations and deterministic merge rules.

use crate::config::{McpConfig, McpServerConfig};
use crate::hashing::bytes_to_hex;
use crate::plugins::PackageMcpServerRegistration;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

/// Origin of an MCP server declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpSourceKind {
    /// Direct config, such as `mcp.servers`.
    Direct,
    /// Extension package manifest declaration.
    Package,
    /// Reserved source kind for future providers.
    Future,
}

impl McpSourceKind {
    fn precedence(self) -> u8 {
        match self {
            Self::Direct => 30,
            Self::Package => 20,
            Self::Future => 10,
        }
    }
}

/// Metadata preserved for every MCP declaration source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpSourceMetadata {
    /// Source kind.
    pub kind: McpSourceKind,
    /// Stable reference to the source location.
    pub source_ref: String,
    /// Whether this source is enabled.
    pub enabled: bool,
    /// Hash of the server config used for change detection.
    pub config_hash: String,
    /// Package id for package-sourced servers.
    pub package_id: Option<String>,
    /// Path for package or config backed sources when available.
    pub config_path: Option<PathBuf>,
}

/// MCP server declaration plus its source metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerSource {
    /// Runtime server id.
    pub server_id: String,
    /// Source metadata.
    pub metadata: McpSourceMetadata,
    /// Server config for this declaration.
    pub config: McpServerConfig,
}

/// Merged source selected for a server id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedMcpServerSource {
    /// Selected declaration after precedence resolution.
    pub selected: McpServerSource,
    /// Lower-precedence declarations that were overridden.
    pub overridden_sources: Vec<McpSourceMetadata>,
}

/// Deterministic merged MCP source set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpSourceSet {
    /// Merged sources keyed by server id.
    pub servers: BTreeMap<String, MergedMcpServerSource>,
}

/// Source merge failure.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum McpSourceMergeError {
    /// Two sources with the same precedence declared the same server id.
    #[error("duplicate MCP server source '{server_id}' at precedence {precedence}")]
    DuplicateSamePrecedence {
        /// Duplicated server id.
        server_id: String,
        /// Source precedence where the duplicate occurred.
        precedence: u8,
        /// Conflicting source metadata.
        sources: Vec<McpSourceMetadata>,
    },
}

impl McpServerSource {
    /// Build a direct config source.
    pub fn direct(server_id: impl Into<String>, config: McpServerConfig, enabled: bool) -> Self {
        let server_id = server_id.into();
        Self {
            metadata: McpSourceMetadata {
                kind: McpSourceKind::Direct,
                source_ref: format!("config:mcp.servers.{server_id}"),
                enabled: enabled && config.enabled,
                config_hash: hash_config(&config),
                package_id: None,
                config_path: None,
            },
            server_id,
            config,
        }
    }

    /// Build a package source.
    pub fn package(registration: &PackageMcpServerRegistration) -> Self {
        let server_id = registration.asset_id.clone();
        Self {
            metadata: McpSourceMetadata {
                kind: McpSourceKind::Package,
                source_ref: format!(
                    "package:{}:{}",
                    registration.package_id, registration.asset_id
                ),
                enabled: registration.config.enabled,
                config_hash: hash_config(&registration.config),
                package_id: Some(registration.package_id.clone()),
                config_path: Some(registration.package_root.clone()),
            },
            server_id,
            config: registration.config.clone(),
        }
    }

    /// Build a future provider source.
    pub fn future(
        server_id: impl Into<String>,
        source_ref: impl Into<String>,
        config: McpServerConfig,
    ) -> Self {
        let server_id = server_id.into();
        Self {
            metadata: McpSourceMetadata {
                kind: McpSourceKind::Future,
                source_ref: source_ref.into(),
                enabled: config.enabled,
                config_hash: hash_config(&config),
                package_id: None,
                config_path: None,
            },
            server_id,
            config,
        }
    }
}

impl McpSourceSet {
    /// Return merged sources in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MergedMcpServerSource)> {
        self.servers.iter()
    }

    /// Get a merged source by server id.
    pub fn get(&self, server_id: &str) -> Option<&MergedMcpServerSource> {
        self.servers.get(server_id)
    }
}

/// Convert direct MCP config into source declarations.
pub fn direct_config_sources(config: &McpConfig) -> Vec<McpServerSource> {
    let mut sources = config
        .servers
        .iter()
        .map(|(server_id, server_config)| {
            McpServerSource::direct(server_id.clone(), server_config.clone(), config.enabled)
        })
        .collect::<Vec<_>>();
    sources.sort_by(|a, b| a.server_id.cmp(&b.server_id));
    sources
}

/// Convert package registrations into source declarations.
pub fn package_sources<'a>(
    registrations: impl IntoIterator<Item = &'a PackageMcpServerRegistration>,
) -> Vec<McpServerSource> {
    let mut sources = registrations
        .into_iter()
        .map(McpServerSource::package)
        .collect::<Vec<_>>();
    sources.sort_by(|a, b| a.server_id.cmp(&b.server_id));
    sources
}

/// Merge MCP sources with explicit precedence.
pub fn merge_mcp_sources(
    sources: impl IntoIterator<Item = McpServerSource>,
) -> Result<McpSourceSet, McpSourceMergeError> {
    let mut by_server: BTreeMap<String, Vec<McpServerSource>> = BTreeMap::new();
    for source in sources {
        by_server
            .entry(source.server_id.clone())
            .or_default()
            .push(source);
    }

    let mut merged = BTreeMap::new();
    for (server_id, mut candidates) in by_server {
        candidates.sort_by(|a, b| {
            b.metadata
                .kind
                .precedence()
                .cmp(&a.metadata.kind.precedence())
                .then_with(|| a.metadata.source_ref.cmp(&b.metadata.source_ref))
        });

        let top_precedence = candidates[0].metadata.kind.precedence();
        let same_precedence = candidates
            .iter()
            .filter(|candidate| candidate.metadata.kind.precedence() == top_precedence)
            .collect::<Vec<_>>();
        if same_precedence.len() > 1 {
            return Err(McpSourceMergeError::DuplicateSamePrecedence {
                server_id,
                precedence: top_precedence,
                sources: same_precedence
                    .into_iter()
                    .map(|candidate| candidate.metadata.clone())
                    .collect(),
            });
        }

        let selected = candidates.remove(0);
        let overridden_sources = candidates
            .into_iter()
            .map(|candidate| candidate.metadata)
            .collect();
        merged.insert(
            selected.server_id.clone(),
            MergedMcpServerSource {
                selected,
                overridden_sources,
            },
        );
    }

    Ok(McpSourceSet { servers: merged })
}

fn hash_config(config: &McpServerConfig) -> String {
    let bytes = serde_json::to_vec(config).unwrap_or_else(|_| format!("{config:?}").into_bytes());
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    bytes_to_hex(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::PackageMcpServerRegistration;
    use std::path::PathBuf;

    #[test]
    fn mcp_source_merge_direct_overrides_package_with_metadata() {
        let package = McpServerSource::package(&PackageMcpServerRegistration {
            package_id: "pkg.docs".to_string(),
            asset_id: "docs".to_string(),
            package_root: PathBuf::from("/tmp/pkg.docs"),
            config: McpServerConfig::stdio("package-docs", Vec::new()),
        });
        let direct = McpServerSource::direct(
            "docs",
            McpServerConfig::stdio("direct-docs", Vec::new()),
            true,
        );

        let source_set = merge_mcp_sources([package, direct]).expect("sources merge");
        let merged = source_set.get("docs").expect("docs source");

        assert_eq!(merged.selected.metadata.kind, McpSourceKind::Direct);
        assert_eq!(merged.overridden_sources.len(), 1);
        assert_eq!(merged.overridden_sources[0].kind, McpSourceKind::Package);
        assert_eq!(
            merged.overridden_sources[0].package_id.as_deref(),
            Some("pkg.docs")
        );
    }

    #[test]
    fn mcp_source_merge_same_precedence_duplicate_fails() {
        let left = McpServerSource::direct(
            "docs",
            McpServerConfig::stdio("left-docs", Vec::new()),
            true,
        );
        let right = McpServerSource {
            metadata: McpSourceMetadata {
                source_ref: "config:alternate.docs".to_string(),
                ..left.metadata.clone()
            },
            ..left.clone()
        };

        let err = merge_mcp_sources([left, right]).expect_err("duplicate should fail");

        assert!(matches!(
            err,
            McpSourceMergeError::DuplicateSamePrecedence { server_id, .. }
                if server_id == "docs"
        ));
    }

    #[test]
    fn mcp_source_merge_disabled_source_is_preserved_as_disabled() {
        let mut server = McpServerConfig::stdio("docs", Vec::new());
        server.enabled = false;

        let source_set =
            merge_mcp_sources([McpServerSource::direct("docs", server, true)]).expect("merge");

        assert!(
            !source_set
                .get("docs")
                .expect("docs source")
                .selected
                .metadata
                .enabled
        );
    }
}
