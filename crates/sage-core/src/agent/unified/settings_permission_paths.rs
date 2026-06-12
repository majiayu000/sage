//! Path and glob helpers for settings-backed permission checks.

use crate::tools::permission::PermissionCache;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

pub(super) fn workspace_relative_path(path: &str, working_dir: &Path) -> String {
    let working_dir = absolute_working_dir(working_dir);
    let path = absolute_permission_path(path, &working_dir);

    if path.is_absolute() {
        return path
            .strip_prefix(&working_dir)
            .map(|relative| relative.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
    }

    path.to_string_lossy().to_string()
}

pub(super) fn glob_search_overlaps_deny_rule(request_key: &str, deny_rule: &str) -> bool {
    let Some(request_arg) = permission_key_argument(request_key, "Glob") else {
        return false;
    };
    let Some(deny_arg) = permission_key_argument(deny_rule, "Glob") else {
        return false;
    };

    if PermissionCache::pattern_matches(
        &format!("Glob({})", deny_arg),
        &format!("Glob({})", request_arg),
    ) {
        return true;
    }

    let request_arg = normalize_permission_pattern(request_arg);
    let deny_arg = normalize_permission_pattern(deny_arg);
    let request_root = literal_directory_prefix_before_glob(&request_arg);
    let deny_root = literal_directory_prefix_before_glob(&deny_arg);

    if deny_root.is_empty() {
        return false;
    }

    let request_key = format!("Glob({})", request_arg);
    let deny_root_key = format!("Glob({})", deny_root);
    if PermissionCache::pattern_matches(&request_key, &deny_root_key) {
        return true;
    }

    if path_is_at_or_under(&request_root, &deny_root) {
        return true;
    }

    request_arg.contains("**") && path_may_contain(&request_root, &deny_root)
}

fn permission_key_argument<'a>(key: &'a str, expected_tool: &str) -> Option<&'a str> {
    let open = key.find('(')?;
    let close = key.rfind(')')?;
    if close <= open {
        return None;
    }

    let tool_name = key[..open].trim();
    if !tool_name.eq_ignore_ascii_case(expected_tool) {
        return None;
    }

    Some(&key[open + 1..close])
}

fn normalize_permission_pattern(pattern: &str) -> String {
    let normalized = normalize_permission_path(Path::new(pattern));
    let mut text = normalized.to_string_lossy().to_string();
    while let Some(stripped) = text.strip_prefix("./") {
        text = stripped.to_string();
    }
    if text.len() > 1 {
        text = text.trim_end_matches('/').to_string();
    }
    text
}

fn literal_directory_prefix_before_glob(pattern: &str) -> String {
    let Some(index) = pattern.find(['*', '?', '[', '{']) else {
        return pattern.to_string();
    };

    let literal = &pattern[..index];
    if literal.ends_with('/') {
        return literal.trim_end_matches('/').to_string();
    }

    literal
        .rsplit_once('/')
        .map(|(directory, _)| directory.to_string())
        .unwrap_or_default()
}

fn path_is_at_or_under(path: &str, root: &str) -> bool {
    if root.is_empty() {
        return true;
    }
    path == root || path.starts_with(&format!("{}/", root))
}

fn path_may_contain(root: &str, path: &str) -> bool {
    root.is_empty() || path == root || path.starts_with(&format!("{}/", root))
}

fn absolute_working_dir(working_dir: &Path) -> PathBuf {
    let path = if working_dir.is_absolute() {
        working_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(working_dir)
    };
    let normalized = normalize_permission_path(&path);
    normalized.canonicalize().unwrap_or(normalized)
}

fn absolute_permission_path(path: &str, working_dir: &Path) -> PathBuf {
    let path = Path::new(path);
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        working_dir.join(path)
    };
    let normalized_path = normalize_permission_path(&absolute_path);
    canonicalize_existing_prefix(&normalized_path).unwrap_or(normalized_path)
}

fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = path.canonicalize() {
        return Some(normalize_permission_path(&canonical));
    }

    let mut current = path.to_path_buf();
    let mut missing_components: Vec<OsString> = Vec::new();

    loop {
        if current.exists() {
            let mut resolved = current.canonicalize().ok()?;
            for component in missing_components.iter().rev() {
                resolved.push(component);
            }
            return Some(normalize_permission_path(&resolved));
        }

        if let Some(file_name) = current.file_name() {
            missing_components.push(file_name.to_os_string());
        }

        let parent = current.parent()?;
        if parent == current {
            return None;
        }
        current = parent.to_path_buf();
    }
}

fn normalize_permission_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }

    normalized
}
