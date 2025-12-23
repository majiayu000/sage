//! Python dependency parser (pyproject.toml, requirements.txt)

use std::path::{Path, PathBuf};

use crate::workspace::models::DependencyInfo;

/// Parse dependencies from Python project files
pub fn parse_python_dependencies(root: &Path, info: &mut DependencyInfo) {
    // Try pyproject.toml first
    let pyproject = root.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            // Simple parsing for dependencies
            let mut in_deps = false;
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("dependencies")
                    || line.contains("[tool.poetry.dependencies]")
                {
                    in_deps = true;
                } else if line.starts_with('[') && !line.contains("dependencies") {
                    in_deps = false;
                } else if in_deps && !line.is_empty() && !line.starts_with('#') {
                    if let Some(name) = line
                        .split('=')
                        .next()
                        .map(|s| s.trim().trim_matches('"').to_string())
                    {
                        if !name.starts_with('[') && name != "python" {
                            info.dependencies.push(name);
                        }
                    }
                }
            }
        }
    }

    // Try requirements.txt
    let requirements = root.join("requirements.txt");
    if requirements.exists() {
        if let Ok(content) = std::fs::read_to_string(&requirements) {
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') && !line.starts_with('-') {
                    let name = line
                        .split(['=', '<', '>', '['])
                        .next()
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        info.dependencies.push(name);
                    }
                }
            }
        }
    }

    // Check for lock files
    let lock_files = ["poetry.lock", "Pipfile.lock", "pdm.lock", "uv.lock"];
    for lock in lock_files {
        if root.join(lock).exists() {
            info.has_lock_file = true;
            info.lock_file = Some(PathBuf::from(lock));
            break;
        }
    }
}
