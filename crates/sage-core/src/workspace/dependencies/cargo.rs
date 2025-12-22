//! Cargo.toml dependency parser for Rust projects

use std::path::{Path, PathBuf};

use crate::workspace::models::DependencyInfo;

/// Parse dependencies from Cargo.toml
pub fn parse_cargo_dependencies(root: &Path, info: &mut DependencyInfo) {
    let cargo_toml = root.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        // Simple parsing - look for dependencies sections
        let mut in_deps = false;
        let mut in_dev_deps = false;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("[dependencies]") {
                in_deps = true;
                in_dev_deps = false;
            } else if line.starts_with("[dev-dependencies]") {
                in_deps = false;
                in_dev_deps = true;
            } else if line.starts_with('[') {
                in_deps = false;
                in_dev_deps = false;
            } else if !line.is_empty() && !line.starts_with('#') {
                if let Some(name) = line.split('=').next().map(|s| s.trim().to_string()) {
                    if in_deps {
                        info.dependencies.push(name);
                    } else if in_dev_deps {
                        info.dev_dependencies.push(name);
                    }
                }
            }
        }
    }

    // Check for lock file
    let lock = root.join("Cargo.lock");
    if lock.exists() {
        info.has_lock_file = true;
        info.lock_file = Some(PathBuf::from("Cargo.lock"));
    }
}
