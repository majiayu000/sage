//! Extension package errors.

use super::package_manifest::{PackageAssetKind, PackageManifestError};
use std::path::PathBuf;

/// Result type for extension package operations.
pub type PackageResult<T> = Result<T, PackageError>;

/// Structured errors returned by the extension package lifecycle.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    /// Manifest parsing or validation failed.
    #[error(transparent)]
    Manifest(#[from] PackageManifestError),

    /// Filesystem operation failed.
    #[error("extension package storage error: {0}")]
    Storage(String),

    /// Package was not found in the installed store.
    #[error("extension package not found: {package_id}")]
    NotFound { package_id: String },

    /// Package is already installed.
    #[error("extension package already installed: {package_id}")]
    AlreadyInstalled { package_id: String },

    /// Package dependency is required but not installed.
    #[error("extension package '{package_id}' missing dependency '{dependency_id}'")]
    MissingDependency {
        package_id: String,
        dependency_id: String,
    },

    /// A declared asset path does not exist under the installed package root.
    #[error("extension package '{package_id}' asset '{asset_id}' is missing path '{path}'")]
    MissingAsset {
        package_id: String,
        asset_id: String,
        path: PathBuf,
    },

    /// A declared asset canonicalizes outside the package root.
    #[error(
        "extension package '{package_id}' asset '{asset_id}' path '{path}' escapes package root"
    )]
    PathEscape {
        package_id: String,
        asset_id: String,
        path: PathBuf,
    },

    /// A runtime registry already has an entry for the asset id.
    #[error(
        "extension package '{package_id}' {asset_kind} asset '{asset_id}' conflicts with existing source '{existing_source}'"
    )]
    RegistryConflict {
        package_id: String,
        asset_kind: PackageAssetKind,
        asset_id: String,
        existing_source: String,
    },

    /// Package cannot perform the requested lifecycle transition.
    #[error("extension package '{package_id}' cannot {action} from state '{state}'")]
    InvalidState {
        package_id: String,
        state: String,
        action: &'static str,
    },

    /// Disable/uninstall could not revoke every registry entry.
    #[error("extension package '{package_id}' partial unregister failure: {}", failures.join("; "))]
    PartialUnregister {
        package_id: String,
        failures: Vec<String>,
    },

    /// Lifecycle transition changed either durable state or runtime state but could not restore both.
    #[error(
        "extension package '{package_id}' partial lifecycle failure while {action} from stored state '{stored_state}': {}",
        failures.join("; ")
    )]
    PartialLifecycle {
        package_id: String,
        action: &'static str,
        stored_state: String,
        failures: Vec<String>,
    },

    /// A registry mutation failed unexpectedly.
    #[error("extension package '{package_id}' registry error: {message}")]
    Registry { package_id: String, message: String },
}

impl PackageError {
    pub(crate) fn store_failure(message: impl Into<String>) -> Self {
        Self::Storage(message.into())
    }
}
