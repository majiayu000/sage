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
        PermissionAction::Filesystem => filesystem_structured_permission_key(profile, input)
            .map(|key| vec![key])
            .unwrap_or_default(),
        PermissionAction::Network => input
            .network_target
            .as_ref()
            .map(|target| vec![format!("{}({})", input.tool_name, normalize_url(target))])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn filesystem_structured_permission_key(
    profile: &PermissionProfile,
    input: &PermissionDecisionInput,
) -> Option<String> {
    let path = input.path.as_deref()?;
    let working_directory = input.working_directory.as_deref();
    let normalized_path = normalize_path(path, working_directory);

    let path_argument = profile
        .filesystem
        .workspace_roots
        .iter()
        .map(|root| normalize_path(root, None))
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
        .map(|(_, relative)| relative)
        .unwrap_or_else(|| {
            permission_path_string(&normalize_permission_key_path(path, working_directory))
        });

    Some(format!("{}({})", input.tool_name, path_argument))
}

pub(super) fn path_is_at_or_under(path: &Path, root: &Path) -> bool {
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

fn normalize_url(url: &str) -> String {
    let trimmed = url.trim();
    let Ok(mut parsed) = reqwest::Url::parse(trimmed) else {
        return trimmed.to_string();
    };

    if !matches!(parsed.scheme(), "http" | "https") {
        return trimmed.to_string();
    }

    if let Some(host) = parsed.host_str() {
        let lowercase_host = host.to_ascii_lowercase();
        if parsed.set_host(Some(&lowercase_host)).is_err() {
            return trimmed.to_string();
        }
    }
    parsed.set_fragment(None);

    if matches!(
        (parsed.scheme(), parsed.port()),
        ("http", Some(80)) | ("https", Some(443))
    ) {
        let _ = parsed.set_port(None);
    }

    parsed.to_string()
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
