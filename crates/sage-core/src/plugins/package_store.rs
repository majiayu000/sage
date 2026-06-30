//! Extension package filesystem store.

use super::package_error::{PackageError, PackageResult};
use super::package_manifest::{
    EXTENSION_MANIFEST_FILE, ExtensionPackageManifest, PackageAssetPath, validate_package_id,
};
use crate::config::default_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const PACKAGE_RECORD_FILE: &str = ".sage-extension-record.toml";

/// Installed extension package state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstalledPackageState {
    /// Installed but not currently enabled.
    Disabled,
    /// Enabled and registered in runtime registries.
    Enabled,
}

/// Package found during discovery before installation.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredExtensionPackage {
    /// Filesystem package root.
    pub package_root: PathBuf,
    /// Parsed package manifest.
    pub manifest: ExtensionPackageManifest,
}

/// Persisted installed extension package record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledPackageRecord {
    /// Package id.
    pub package_id: String,
    /// Package version.
    pub version: String,
    /// Installed package root.
    pub install_root: PathBuf,
    /// Current lifecycle state.
    pub state: InstalledPackageState,
    /// Manifest captured at install time.
    pub manifest: ExtensionPackageManifest,
}

impl InstalledPackageRecord {
    /// Returns whether the package is enabled.
    pub fn enabled(&self) -> bool {
        self.state == InstalledPackageState::Enabled
    }
}

/// Managed extension package store.
#[derive(Debug, Clone)]
pub struct ExtensionPackageStore {
    root: PathBuf,
}

impl ExtensionPackageStore {
    /// Create a package store rooted at a specific directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Create the default user package store.
    pub fn default_user_store() -> PackageResult<Self> {
        Ok(Self::new(
            default_data_dir()
                .map_err(|err| PackageError::store_failure(err.to_string()))?
                .join("extensions")
                .join("packages"),
        ))
    }

    /// Store root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Discover package manifests from roots and their direct children.
    pub fn discover(&self, roots: &[PathBuf]) -> PackageResult<Vec<DiscoveredExtensionPackage>> {
        let mut packages = Vec::new();
        for root in roots {
            if !root.exists() {
                continue;
            }
            if manifest_path(root).is_file() {
                packages.push(read_discovered_package(root)?);
                continue;
            }
            for entry in
                fs::read_dir(root).map_err(|err| PackageError::store_failure(err.to_string()))?
            {
                let entry = entry.map_err(|err| PackageError::store_failure(err.to_string()))?;
                let path = entry.path();
                if entry
                    .file_type()
                    .map_err(|err| PackageError::store_failure(err.to_string()))?
                    .is_dir()
                    && manifest_path(&path).is_file()
                {
                    packages.push(read_discovered_package(&path)?);
                }
            }
        }
        packages.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
        Ok(packages)
    }

    /// Read a manifest from an arbitrary package root.
    pub fn read_manifest(&self, package_root: &Path) -> PackageResult<ExtensionPackageManifest> {
        read_manifest_file(&manifest_path(package_root))
    }

    /// Install a package from a source root into the managed store.
    pub fn install(&self, source_root: &Path) -> PackageResult<InstalledPackageRecord> {
        let manifest = self.read_manifest(source_root)?;
        validate_manifest_assets_under_root(&manifest, source_root)?;

        let package_dir = self.package_dir(&manifest.id)?;
        if package_dir.exists() {
            return Err(PackageError::AlreadyInstalled {
                package_id: manifest.id.clone(),
            });
        }

        fs::create_dir_all(&self.root)
            .map_err(|err| PackageError::store_failure(err.to_string()))?;
        let tmp_dir = self.root.join(format!(".tmp-{}", manifest.id));
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)
                .map_err(|err| PackageError::store_failure(err.to_string()))?;
        }
        copy_package_dir(source_root, &tmp_dir)?;

        let record = InstalledPackageRecord {
            package_id: manifest.id.clone(),
            version: manifest.version.clone(),
            install_root: package_dir.clone(),
            state: InstalledPackageState::Disabled,
            manifest,
        };

        write_record_at(&tmp_dir, &record)?;
        fs::rename(&tmp_dir, &package_dir)
            .map_err(|err| PackageError::store_failure(err.to_string()))?;
        Ok(record)
    }

    /// List installed package records.
    pub fn list(&self) -> PackageResult<Vec<InstalledPackageRecord>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in
            fs::read_dir(&self.root).map_err(|err| PackageError::store_failure(err.to_string()))?
        {
            let entry = entry.map_err(|err| PackageError::store_failure(err.to_string()))?;
            if entry
                .file_type()
                .map_err(|err| PackageError::store_failure(err.to_string()))?
                .is_dir()
            {
                let record_path = entry.path().join(PACKAGE_RECORD_FILE);
                if record_path.is_file() {
                    records.push(read_record_file(&record_path)?);
                }
            }
        }
        records.sort_by(|left, right| left.package_id.cmp(&right.package_id));
        Ok(records)
    }

    /// Read one installed package.
    pub fn read(&self, package_id: &str) -> PackageResult<InstalledPackageRecord> {
        let package_dir = self.package_dir(package_id)?;
        let record_path = package_dir.join(PACKAGE_RECORD_FILE);
        if !record_path.exists() {
            return Err(PackageError::NotFound {
                package_id: package_id.to_string(),
            });
        }
        self.ensure_package_dir_inside_root(&package_dir)?;
        read_record_file(&record_path)
    }

    /// Persist a package record.
    pub fn write_record(&self, record: &InstalledPackageRecord) -> PackageResult<()> {
        let package_dir = self.package_dir(&record.package_id)?;
        if record.install_root != package_dir {
            return Err(PackageError::store_failure(format!(
                "package '{}' install root does not match managed store root",
                record.package_id
            )));
        }
        self.ensure_package_dir_inside_root(&package_dir)?;
        write_record_at(&package_dir, record)
    }

    /// Remove an installed package directory.
    pub fn uninstall(&self, package_id: &str) -> PackageResult<()> {
        let package_dir = self.package_dir(package_id)?;
        if !package_dir.exists() {
            return Err(PackageError::NotFound {
                package_id: package_id.to_string(),
            });
        }
        self.ensure_package_dir_inside_root(&package_dir)?;
        fs::remove_dir_all(package_dir).map_err(|err| PackageError::store_failure(err.to_string()))
    }

    /// Returns whether a package id is installed.
    pub fn is_installed(&self, package_id: &str) -> PackageResult<bool> {
        let package_dir = self.package_dir(package_id)?;
        Ok(package_dir.join(PACKAGE_RECORD_FILE).is_file())
    }

    fn package_dir(&self, package_id: &str) -> PackageResult<PathBuf> {
        validate_package_id(package_id)?;
        Ok(self.root.join(package_id))
    }

    fn ensure_package_dir_inside_root(&self, package_dir: &Path) -> PackageResult<()> {
        let canonical_root = self
            .root
            .canonicalize()
            .map_err(|err| PackageError::store_failure(err.to_string()))?;
        let canonical_package = package_dir
            .canonicalize()
            .map_err(|err| PackageError::store_failure(err.to_string()))?;
        if canonical_package.starts_with(&canonical_root) {
            Ok(())
        } else {
            Err(PackageError::store_failure(format!(
                "package directory '{}' escapes store root '{}'",
                canonical_package.display(),
                canonical_root.display()
            )))
        }
    }
}

/// Validate that all declared package asset files exist inside package root.
pub fn validate_manifest_assets_under_root(
    manifest: &ExtensionPackageManifest,
    package_root: &Path,
) -> PackageResult<()> {
    let canonical_root = package_root
        .canonicalize()
        .map_err(|err| PackageError::store_failure(err.to_string()))?;

    for asset_path in manifest.asset_paths() {
        validate_asset_path_exists(manifest, &canonical_root, &asset_path)?;
    }
    Ok(())
}

fn validate_asset_path_exists(
    manifest: &ExtensionPackageManifest,
    canonical_root: &Path,
    asset_path: &PackageAssetPath,
) -> PackageResult<()> {
    let full_path = canonical_root.join(&asset_path.path);
    if !full_path.exists() {
        return Err(PackageError::MissingAsset {
            package_id: manifest.id.clone(),
            asset_id: asset_path.asset_id.clone(),
            path: asset_path.path.clone(),
        });
    }

    let canonical_asset = full_path
        .canonicalize()
        .map_err(|err| PackageError::store_failure(err.to_string()))?;
    if !canonical_asset.starts_with(canonical_root) {
        return Err(PackageError::PathEscape {
            package_id: manifest.id.clone(),
            asset_id: asset_path.asset_id.clone(),
            path: asset_path.path.clone(),
        });
    }
    Ok(())
}

fn read_discovered_package(package_root: &Path) -> PackageResult<DiscoveredExtensionPackage> {
    Ok(DiscoveredExtensionPackage {
        package_root: package_root.to_path_buf(),
        manifest: read_manifest_file(&manifest_path(package_root))?,
    })
}

fn manifest_path(package_root: &Path) -> PathBuf {
    package_root.join(EXTENSION_MANIFEST_FILE)
}

fn read_manifest_file(path: &Path) -> PackageResult<ExtensionPackageManifest> {
    let contents =
        fs::read_to_string(path).map_err(|err| PackageError::store_failure(err.to_string()))?;
    ExtensionPackageManifest::from_toml_str(&contents).map_err(Into::into)
}

fn read_record_file(path: &Path) -> PackageResult<InstalledPackageRecord> {
    let contents =
        fs::read_to_string(path).map_err(|err| PackageError::store_failure(err.to_string()))?;
    toml::from_str(&contents).map_err(|err| PackageError::store_failure(err.to_string()))
}

fn write_record_at(package_root: &Path, record: &InstalledPackageRecord) -> PackageResult<()> {
    let contents = toml::to_string_pretty(record)
        .map_err(|err| PackageError::store_failure(err.to_string()))?;
    fs::write(package_root.join(PACKAGE_RECORD_FILE), contents)
        .map_err(|err| PackageError::store_failure(err.to_string()))
}

fn copy_package_dir(source: &Path, destination: &Path) -> PackageResult<()> {
    fs::create_dir_all(destination).map_err(|err| PackageError::store_failure(err.to_string()))?;
    for entry in fs::read_dir(source).map_err(|err| PackageError::store_failure(err.to_string()))? {
        let entry = entry.map_err(|err| PackageError::store_failure(err.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|err| PackageError::store_failure(err.to_string()))?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            return Err(PackageError::store_failure(format!(
                "symlink entries are not supported in extension packages: {}",
                entry.path().display()
            )));
        }
        if file_type.is_dir() {
            copy_package_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)
                .map_err(|err| PackageError::store_failure(err.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "package_store_tests.rs"]
mod tests;
