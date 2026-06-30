use super::*;
use crate::plugins::package_manifest::PackageManifestError;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_package(root: &Path, manifest: &str) {
    fs::create_dir_all(root.join("skills/reviewer")).unwrap();
    fs::create_dir_all(root.join("commands")).unwrap();
    fs::write(root.join(EXTENSION_MANIFEST_FILE), manifest).unwrap();
    fs::write(root.join("skills/reviewer/SKILL.md"), "review prompt").unwrap();
    fs::write(root.join("commands/review.md"), "review command").unwrap();
}

fn manifest() -> &'static str {
    r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["skills:read", "commands:run"]

[[assets.skills]]
id = "reviewer"
path = "skills/reviewer/SKILL.md"
required_permissions = ["skills:read"]

[[assets.commands]]
id = "review"
path = "commands/review.md"
required_permissions = ["commands:run"]
"#
}

#[test]
fn package_store_discovers_installs_lists_reads_and_uninstalls() {
    let source = TempDir::new().unwrap();
    let store_root = TempDir::new().unwrap();
    write_package(source.path(), manifest());

    let store = ExtensionPackageStore::new(store_root.path());
    let discovered = store.discover(&[source.path().to_path_buf()]).unwrap();
    assert_eq!(discovered.len(), 1);

    let installed = store.install(source.path()).unwrap();
    assert_eq!(installed.package_id, "acme.review");
    assert_eq!(installed.state, InstalledPackageState::Disabled);

    assert_eq!(store.list().unwrap().len(), 1);
    assert_eq!(store.read("acme.review").unwrap().version, "1.0.0");

    store.uninstall("acme.review").unwrap();
    assert!(store.read("acme.review").is_err());
}

#[test]
fn missing_asset_fails_before_store_mutation() {
    let source = TempDir::new().unwrap();
    let store_root = TempDir::new().unwrap();
    fs::write(source.path().join(EXTENSION_MANIFEST_FILE), manifest()).unwrap();

    let store = ExtensionPackageStore::new(store_root.path());
    let err = store.install(source.path()).unwrap_err();

    assert!(matches!(err, PackageError::MissingAsset { .. }));
    assert!(store.list().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn symlink_escape_fails_before_store_mutation() {
    use std::os::unix::fs::symlink;

    let source = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let store_root = TempDir::new().unwrap();
    fs::create_dir_all(source.path().join("commands")).unwrap();
    fs::write(outside.path().join("review.md"), "outside").unwrap();
    symlink(
        outside.path().join("review.md"),
        source.path().join("commands/review.md"),
    )
    .unwrap();
    fs::write(
        source.path().join(EXTENSION_MANIFEST_FILE),
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
"#,
    )
    .unwrap();

    let store = ExtensionPackageStore::new(store_root.path());
    let err = store.install(source.path()).unwrap_err();

    assert!(matches!(err, PackageError::PathEscape { .. }));
    assert!(store.list().unwrap().is_empty());
}

#[test]
fn public_package_id_path_escape_fails_closed() {
    let store_root = TempDir::new().unwrap();
    let store = ExtensionPackageStore::new(store_root.path());

    assert!(matches!(
        store.read("../outside"),
        Err(PackageError::Manifest(
            PackageManifestError::InvalidField { .. }
        ))
    ));
    assert!(matches!(
        store.uninstall("../outside"),
        Err(PackageError::Manifest(
            PackageManifestError::InvalidField { .. }
        ))
    ));
    assert!(!store.is_installed("../outside"));
}

#[test]
fn dot_only_public_package_id_fails_closed() {
    let store_root = TempDir::new().unwrap();
    let store = ExtensionPackageStore::new(store_root.path());

    assert!(matches!(
        store.read(".."),
        Err(PackageError::Manifest(
            PackageManifestError::InvalidField { .. }
        ))
    ));
    assert!(!store.is_installed("."));
}
