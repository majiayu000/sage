use std::path::{Component, Path, PathBuf};

use crate::thread_store::error::{ThreadStoreError, ThreadStoreResult};

pub(super) struct PayloadDeleteReport {
    pub files_deleted: usize,
    pub errors: Vec<String>,
}

pub(super) fn delete_payload_refs(
    payload_refs: Vec<String>,
    payload_root: Option<&Path>,
) -> PayloadDeleteReport {
    let mut report = PayloadDeleteReport {
        files_deleted: 0,
        errors: Vec::new(),
    };
    for payload_ref in payload_refs {
        match payload_file_path(&payload_ref, payload_root) {
            Ok(Some(path)) => match std::fs::remove_file(&path) {
                Ok(()) => report.files_deleted += 1,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => report.errors.push(format!("{}: {err}", path.display())),
            },
            Ok(None) => {}
            Err(err) => report.errors.push(format!("{payload_ref}: {err}")),
        }
    }
    report
}

pub(super) fn payload_file_path(
    payload_ref: &str,
    payload_root: Option<&Path>,
) -> ThreadStoreResult<Option<PathBuf>> {
    let Some(relative) = payload_ref.strip_prefix("store_payload:") else {
        return Err(invalid_payload_input("payload ref is not store-owned"));
    };
    let root =
        payload_root.ok_or_else(|| invalid_payload_input("payload root is not configured"))?;
    let relative = Path::new(relative);
    validate_relative_payload_path(relative)?;
    let root = root.canonicalize()?;
    let path = root.join(relative);
    if path.symlink_metadata()?.file_type().is_symlink() {
        return Err(invalid_payload_input(format!(
            "payload path is a symlink: {}",
            path.display()
        )));
    }
    let canonical_path = path.canonicalize()?;
    if !canonical_path.starts_with(&root) {
        return Err(invalid_payload_input(format!(
            "payload path escapes store root: {}",
            canonical_path.display()
        )));
    }
    Ok(Some(canonical_path))
}

pub(super) fn validate_relative_payload_path(path: &Path) -> ThreadStoreResult<()> {
    if path.is_absolute() {
        return Err(invalid_payload_input("payload path must be relative"));
    }
    let valid = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if !valid {
        return Err(invalid_payload_input(
            "payload path contains parent or prefix components",
        ));
    }
    Ok(())
}

fn invalid_payload_input(message: impl Into<String>) -> ThreadStoreError {
    ThreadStoreError::InvalidInput(message.into())
}
