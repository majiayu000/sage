use super::{PermissionAction, PermissionDecisionEngine, PermissionDecisionInput};
use crate::permissions::decision_engine_keys::{normalize_path, path_is_at_or_under};
use std::path::{Path, PathBuf};

impl PermissionDecisionEngine {
    pub(super) fn path_is_in_workspace(&self, path: &str, working_directory: Option<&str>) -> bool {
        let path = normalize_path(path, working_directory);
        self.profile
            .filesystem
            .workspace_roots
            .iter()
            .map(|root| normalize_path(root, working_directory))
            .any(|root| path_is_at_or_under(&path, &root))
    }

    pub(super) fn path_is_protected(&self, path: &str, working_directory: Option<&str>) -> bool {
        let path = normalize_path(path, working_directory);
        self.protected_path_roots(working_directory)
            .iter()
            .any(|protected| path_is_at_or_under(&path, protected))
    }

    pub(super) fn search_scope_touches_protected(
        &self,
        input: &PermissionDecisionInput,
        path: &str,
        working_directory: Option<&str>,
    ) -> bool {
        if !matches!(input.action, PermissionAction::Filesystem)
            || !matches!(
                input.tool_name.to_ascii_lowercase().as_str(),
                "grep" | "glob"
            )
        {
            return false;
        }

        let scope = if input.tool_name.eq_ignore_ascii_case("glob") {
            glob_static_scope(path)
        } else {
            PathBuf::from(path)
        };
        let scope = normalize_path(scope, working_directory);
        self.protected_path_roots(working_directory)
            .iter()
            .any(|protected| {
                path_is_at_or_under(&scope, protected) || path_is_at_or_under(protected, &scope)
            })
    }

    fn protected_path_roots(&self, working_directory: Option<&str>) -> Vec<PathBuf> {
        self.profile
            .filesystem
            .protected_paths
            .iter()
            .flat_map(|protected| {
                if Path::new(protected).is_absolute() {
                    vec![normalize_path(protected, None)]
                } else {
                    self.profile
                        .filesystem
                        .workspace_roots
                        .iter()
                        .map(|root| {
                            normalize_path(
                                normalize_path(root, working_directory).join(protected),
                                None,
                            )
                        })
                        .collect()
                }
            })
            .collect()
    }
}

fn glob_static_scope(pattern: &str) -> PathBuf {
    let mut scope = PathBuf::new();
    for component in Path::new(pattern).components() {
        let component_text = component.as_os_str().to_string_lossy();
        if component_text
            .chars()
            .any(|ch| matches!(ch, '*' | '?' | '[' | ']' | '{' | '}'))
        {
            break;
        }
        if component_text == "." {
            continue;
        }
        scope.push(component.as_os_str());
    }

    if scope.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        scope
    }
}
