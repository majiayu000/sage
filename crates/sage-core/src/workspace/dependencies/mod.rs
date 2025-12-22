//! Dependency analysis for different languages

use std::path::Path;

use super::detector::{LanguageType, ProjectType};
use super::models::DependencyInfo;

mod cargo;
mod go;
mod npm;
mod python;

pub use cargo::parse_cargo_dependencies;
pub use go::parse_go_dependencies;
pub use npm::parse_npm_dependencies;
pub use python::parse_python_dependencies;

/// Analyze dependencies based on project type
pub fn analyze_dependencies(root: &Path, project: &ProjectType) -> Option<DependencyInfo> {
    let mut info = DependencyInfo {
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        peer_dependencies: Vec::new(),
        total_count: 0,
        has_lock_file: false,
        lock_file: None,
    };

    match project.primary_language {
        LanguageType::Rust => {
            parse_cargo_dependencies(root, &mut info);
        }
        LanguageType::TypeScript | LanguageType::JavaScript => {
            parse_npm_dependencies(root, &mut info);
        }
        LanguageType::Python => {
            parse_python_dependencies(root, &mut info);
        }
        LanguageType::Go => {
            parse_go_dependencies(root, &mut info);
        }
        _ => return None,
    }

    info.total_count = info.dependencies.len() + info.dev_dependencies.len();
    Some(info)
}
