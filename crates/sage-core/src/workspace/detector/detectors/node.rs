//! Node.js/TypeScript project detection

use crate::workspace::detector::framework_detection::{
    detect_node_frameworks, detect_node_test_frameworks,
};
use crate::workspace::detector::types::{
    BuildSystem, LanguageType, ProjectType, RuntimeType,
};
use std::path::Path;

/// Detects Node.js/TypeScript projects via package.json
pub(super) fn detect(root: &Path, project: &mut ProjectType) {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        // Check for Deno
        detect_deno(root, project);
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::JavaScript;
    }
    project.runtime = Some(RuntimeType::Node);

    // Detect package manager
    detect_package_manager(root, project);

    // Check for TypeScript
    if root.join("tsconfig.json").exists() {
        project.primary_language = LanguageType::TypeScript;
        project.secondary_languages.insert(LanguageType::JavaScript);
    }

    // Parse package.json for frameworks
    if let Ok(content) = std::fs::read_to_string(&package_json) {
        detect_node_frameworks(&content, project);
        detect_node_test_frameworks(&content, project);

        // Check for workspaces
        if content.contains("\"workspaces\"") {
            project.is_workspace = true;
            project.is_monorepo = true;
        }
    }

    // Check for Deno in addition to Node
    detect_deno(root, project);
}

fn detect_package_manager(root: &Path, project: &mut ProjectType) {
    if root.join("yarn.lock").exists() {
        project.build_systems.insert(BuildSystem::Yarn);
    } else if root.join("pnpm-lock.yaml").exists() {
        project.build_systems.insert(BuildSystem::Pnpm);
    } else if root.join("bun.lockb").exists() {
        project.build_systems.insert(BuildSystem::Bun);
        project.runtime = Some(RuntimeType::Bun);
    } else {
        project.build_systems.insert(BuildSystem::Npm);
    }
}

fn detect_deno(root: &Path, project: &mut ProjectType) {
    if root.join("deno.json").exists() || root.join("deno.jsonc").exists() {
        project.runtime = Some(RuntimeType::Deno);
        if project.primary_language == LanguageType::Unknown {
            project.primary_language = LanguageType::TypeScript;
        }
    }
}
