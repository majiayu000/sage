//! Rust project detection

use crate::workspace::detector::types::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, RuntimeType, TestFramework,
};
use std::path::Path;

/// Detects Rust projects via Cargo.toml
pub(super) fn detect(root: &Path, project: &mut ProjectType) {
    let cargo_toml = root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return;
    }

    project.primary_language = LanguageType::Rust;
    project.build_systems.insert(BuildSystem::Cargo);
    project.test_frameworks.insert(TestFramework::RustBuiltin);
    project.runtime = Some(RuntimeType::Native);

    // Check for workspace
    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        if content.contains("[workspace]") {
            project.is_workspace = true;
        }

        // Detect frameworks
        if content.contains("actix") {
            project.frameworks.insert(FrameworkType::Actix);
        }
        if content.contains("axum") {
            project.frameworks.insert(FrameworkType::Axum);
        }
        if content.contains("rocket") {
            project.frameworks.insert(FrameworkType::Rocket);
        }
        if content.contains("tauri") {
            project.frameworks.insert(FrameworkType::Tauri);
        }
    }
}
