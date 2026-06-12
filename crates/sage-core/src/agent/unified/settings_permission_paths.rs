//! Path and glob helpers for settings-backed permission checks.

use crate::tools::permission::PermissionCache;
use std::path::{Component, Path, PathBuf};

pub(super) fn workspace_relative_path(path: &str, working_dir: &Path) -> String {
    let working_dir = absolute_working_dir(working_dir);
    let path = absolute_permission_path(path, &working_dir);

    if path.is_absolute() {
        return path
            .strip_prefix(&working_dir)
            .map(permission_path_string)
            .unwrap_or_else(|_| permission_path_string(&path));
    }

    permission_path_string(&path)
}

pub(super) fn glob_search_overlaps_deny_rule(request_key: &str, deny_rule: &str) -> bool {
    path_search_overlaps_deny_rule(request_key, deny_rule, "Glob", false)
}

pub(super) fn grep_search_overlaps_deny_rule(request_key: &str, deny_rule: &str) -> bool {
    path_search_overlaps_deny_rule(request_key, deny_rule, "Grep", true)
}

fn path_search_overlaps_deny_rule(
    request_key: &str,
    deny_rule: &str,
    tool_name: &str,
    request_covers_descendants: bool,
) -> bool {
    let Some(request_arg) = permission_key_argument(request_key, tool_name) else {
        return false;
    };
    let Some(deny_arg) = permission_key_argument(deny_rule, tool_name) else {
        return false;
    };

    let request_key = format!("{}({})", tool_name, request_arg);
    let deny_key = format!("{}({})", tool_name, deny_arg);
    if PermissionCache::pattern_matches(&deny_key, &request_key) {
        return true;
    }

    let request_arg = normalize_permission_pattern(request_arg);
    let deny_arg = normalize_permission_pattern(deny_arg);
    let request_root = literal_directory_prefix_before_glob(&request_arg);
    let deny_root = literal_directory_prefix_before_glob(&deny_arg);

    if deny_root.is_empty() {
        return false;
    }

    let request_key = format!("{}({})", tool_name, request_arg);
    let deny_root_key = format!("{}({})", tool_name, deny_root);
    if PermissionCache::pattern_matches(&request_key, &deny_root_key) {
        return true;
    }

    if path_is_at_or_under(&request_root, &deny_root) {
        return true;
    }

    (request_covers_descendants || request_arg.contains("**"))
        && path_may_contain(&request_root, &deny_root)
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
    let mut text = permission_path_string(&normalized);
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
    let resolved_path = canonicalize_existing_components(&absolute_path);
    normalize_permission_path(&resolved_path)
}

fn canonicalize_existing_components(path: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();

    for component in path.components() {
        if matches!(component, Component::CurDir) {
            continue;
        }

        resolved.push(component.as_os_str());
        if let Ok(canonical) = resolved.canonicalize() {
            resolved = canonical;
        }
    }

    resolved
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

fn permission_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
