//! Python project detection

use crate::workspace::detector::types::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, RuntimeType, TestFramework,
};
use std::path::Path;

/// Detects Python projects via pyproject.toml, requirements.txt, or setup.py
pub(super) fn detect(root: &Path, project: &mut ProjectType) {
    detect_pyproject(root, project);
    detect_requirements(root, project);
    detect_setup_py(root, project);
    detect_pytest(root, project);
}

fn detect_pyproject(root: &Path, project: &mut ProjectType) {
    let pyproject = root.join("pyproject.toml");
    if !pyproject.exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Python;
    }
    project.runtime = Some(RuntimeType::Python);

    if let Ok(content) = std::fs::read_to_string(&pyproject) {
        // Detect build system
        if content.contains("[tool.poetry]") {
            project.build_systems.insert(BuildSystem::Poetry);
        } else if content.contains("[tool.pdm]") {
            project.build_systems.insert(BuildSystem::Pdm);
        } else if content.contains("[tool.uv]") || root.join("uv.lock").exists() {
            project.build_systems.insert(BuildSystem::Uv);
        } else {
            project.build_systems.insert(BuildSystem::Pip);
        }

        // Detect frameworks
        if content.contains("django") {
            project.frameworks.insert(FrameworkType::Django);
        }
        if content.contains("flask") {
            project.frameworks.insert(FrameworkType::Flask);
        }
        if content.contains("fastapi") {
            project.frameworks.insert(FrameworkType::FastApi);
        }

        // Detect test frameworks
        if content.contains("pytest") {
            project.test_frameworks.insert(TestFramework::Pytest);
        }
    }
}

fn detect_requirements(root: &Path, project: &mut ProjectType) {
    if !root.join("requirements.txt").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Python;
    }
    project.build_systems.insert(BuildSystem::Pip);
    project.runtime = Some(RuntimeType::Python);
}

fn detect_setup_py(root: &Path, project: &mut ProjectType) {
    if !root.join("setup.py").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Python;
    }
    project.build_systems.insert(BuildSystem::Pip);
}

fn detect_pytest(root: &Path, project: &mut ProjectType) {
    if root.join("pytest.ini").exists() || root.join("conftest.py").exists() {
        project.test_frameworks.insert(TestFramework::Pytest);
    }
}
