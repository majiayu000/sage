use super::decision_engine::{PermissionAction, PermissionDecisionInput};
use super::profile::PermissionProfile;
use std::path::{Component, Path, PathBuf};

pub(super) fn rule_match_keys(
    profile: &PermissionProfile,
    input: &PermissionDecisionInput,
) -> Vec<String> {
    if !input.permission_keys.is_empty() {
        return input.permission_keys.clone();
    }

    match input.action {
        PermissionAction::Filesystem => filesystem_structured_permission_keys(profile, input),
        PermissionAction::Network => input
            .network_target
            .as_ref()
            .map(|target| {
                normalize_url_keys(target)
                    .into_iter()
                    .map(|target| format!("{}({})", input.tool_name, target))
                    .collect()
            })
            .unwrap_or_default(),
        _ => bare_tool_key(input),
    }
}

fn filesystem_structured_permission_keys(
    profile: &PermissionProfile,
    input: &PermissionDecisionInput,
) -> Vec<String> {
    let Some(path) = input.path.as_deref() else {
        return Vec::new();
    };
    let working_directory = input.working_directory.as_deref();
    let normalized_path = normalize_path(path, working_directory);
    let mut path_arguments = Vec::new();

    if let Some((_, relative)) = profile
        .filesystem
        .workspace_roots
        .iter()
        .map(|root| normalize_path(root, working_directory))
        .filter_map(|root| {
            normalized_path.strip_prefix(&root).ok().map(|relative| {
                (
                    root.components().count(),
                    permission_path_string(if relative.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        relative
                    }),
                )
            })
        })
        .max_by_key(|(component_count, _)| *component_count)
    {
        push_unique(&mut path_arguments, relative);
    }
    push_path_aliases(&mut path_arguments, &normalized_path);
    push_path_aliases(
        &mut path_arguments,
        &normalize_permission_key_path(path, working_directory),
    );

    path_arguments
        .into_iter()
        .map(|path| format!("{}({})", input.tool_name, path))
        .collect()
}

fn bare_tool_key(input: &PermissionDecisionInput) -> Vec<String> {
    if input.tool_name.is_empty() {
        Vec::new()
    } else {
        vec![input.tool_name.clone()]
    }
}

pub(super) fn path_is_at_or_under(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        let path = permission_path_string(path).to_ascii_lowercase();
        let root = permission_path_string(root).to_ascii_lowercase();
        return path == root
            || path
                .strip_prefix(&root)
                .is_some_and(|suffix| suffix.starts_with('/'));
    }

    path == root || path.starts_with(root)
}

pub(super) fn normalize_path(path: impl AsRef<Path>, working_directory: Option<&str>) -> PathBuf {
    let path = path.as_ref();
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(working_directory) = working_directory {
        normalize_path(working_directory, None).join(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    canonicalize_existing_components(&absolute)
}

fn permission_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn push_path_aliases(values: &mut Vec<String>, path: &Path) {
    let path = permission_path_string(path);
    push_unique(values, path.clone());
    if let Some(stripped) = path.strip_prefix("/private/") {
        push_unique(values, format!("/{}", stripped));
    }
}

fn normalize_permission_key_path(path: &str, working_directory: Option<&str>) -> PathBuf {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(working_directory) = working_directory {
        Path::new(working_directory).join(path)
    } else {
        path.to_path_buf()
    };
    normalize_lexical_path(&absolute)
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
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

fn normalize_url_keys(url: &str) -> Vec<String> {
    let trimmed = url.trim();
    let Ok(mut parsed) = reqwest::Url::parse(trimmed) else {
        return vec![trimmed.to_string()];
    };

    if !matches!(parsed.scheme(), "http" | "https") {
        return vec![trimmed.to_string()];
    }

    if let Some(host) = parsed.host_str() {
        let lowercase_host = host.to_ascii_lowercase();
        if parsed.set_host(Some(&lowercase_host)).is_err() {
            return vec![trimmed.to_string()];
        }
    }
    if parsed.set_username("").is_err() || parsed.set_password(None).is_err() {
        return vec![trimmed.to_string()];
    }
    parsed.set_fragment(None);

    if matches!(
        (parsed.scheme(), parsed.port()),
        ("http", Some(80)) | ("https", Some(443))
    ) {
        let _ = parsed.set_port(None);
    }

    let normalized = parsed.to_string();
    let mut keys = Vec::new();
    if parsed.path() == "/" && parsed.query().is_none() {
        push_unique(&mut keys, normalized.trim_end_matches('/').to_string());
    }
    push_unique(&mut keys, normalized);
    keys
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn canonicalize_existing_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }

        if let Ok(canonical) = normalized.canonicalize() {
            normalized = canonical;
        }
    }

    normalized
}
