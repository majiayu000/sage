//! package.json dependency parser for JavaScript/TypeScript projects

use std::path::{Path, PathBuf};

use crate::workspace::models::DependencyInfo;

/// Parse dependencies from package.json
pub fn parse_npm_dependencies(root: &Path, info: &mut DependencyInfo) {
    let package_json = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&package_json) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                info.dependencies = deps.keys().cloned().collect();
            }
            if let Some(deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
                info.dev_dependencies = deps.keys().cloned().collect();
            }
            if let Some(deps) = json.get("peerDependencies").and_then(|d| d.as_object()) {
                info.peer_dependencies = deps.keys().cloned().collect();
            }
        }
    }

    // Check for lock files
    let lock_files = [
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "bun.lockb",
    ];
    for lock in lock_files {
        if root.join(lock).exists() {
            info.has_lock_file = true;
            info.lock_file = Some(PathBuf::from(lock));
            break;
        }
    }
}
