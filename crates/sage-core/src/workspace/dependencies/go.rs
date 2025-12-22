//! go.mod dependency parser for Go projects

use std::path::{Path, PathBuf};

use crate::workspace::models::DependencyInfo;

/// Parse dependencies from go.mod
pub fn parse_go_dependencies(root: &Path, info: &mut DependencyInfo) {
    let go_mod = root.join("go.mod");
    if let Ok(content) = std::fs::read_to_string(&go_mod) {
        let mut in_require = false;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("require (") {
                in_require = true;
            } else if line == ")" {
                in_require = false;
            } else if in_require || line.starts_with("require ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim_start_matches("require ");
                    info.dependencies.push(name.to_string());
                }
            }
        }
    }

    // Check for lock file
    if root.join("go.sum").exists() {
        info.has_lock_file = true;
        info.lock_file = Some(PathBuf::from("go.sum"));
    }
}
