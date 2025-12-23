//! Other languages and monorepo detection

use crate::workspace::detector::types::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, RuntimeType, TestFramework,
};
use std::path::Path;

/// Detects other languages and frameworks (Ruby, C/C++, C#, PHP, monorepos)
pub(super) fn detect(root: &Path, project: &mut ProjectType) {
    detect_ruby(root, project);
    detect_cpp(root, project);
    detect_csharp(root, project);
    detect_php(root, project);
    detect_monorepo(root, project);
}

fn detect_ruby(root: &Path, project: &mut ProjectType) {
    if !root.join("Gemfile").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Ruby;
    }

    if let Ok(content) = std::fs::read_to_string(root.join("Gemfile")) {
        if content.contains("rails") {
            project.frameworks.insert(FrameworkType::Rails);
        }
        if content.contains("rspec") {
            project.test_frameworks.insert(TestFramework::RSpec);
        }
    }
}

fn detect_cpp(root: &Path, project: &mut ProjectType) {
    // CMake
    if root.join("CMakeLists.txt").exists() {
        project.build_systems.insert(BuildSystem::CMake);
        if project.primary_language == LanguageType::Unknown {
            project.primary_language = LanguageType::Cpp;
        }
        project.runtime = Some(RuntimeType::Native);
    }

    // Make
    if root.join("Makefile").exists() {
        project.build_systems.insert(BuildSystem::Make);
    }
}

fn detect_csharp(root: &Path, project: &mut ProjectType) {
    if !root.join("*.csproj").exists() && !root.join("*.sln").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::CSharp;
    }
    project.runtime = Some(RuntimeType::DotNet);
}

fn detect_php(root: &Path, project: &mut ProjectType) {
    if !root.join("composer.json").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Php;
    }

    if let Ok(content) = std::fs::read_to_string(root.join("composer.json")) {
        if content.contains("phpunit") {
            project.test_frameworks.insert(TestFramework::PHPUnit);
        }
    }
}

fn detect_monorepo(root: &Path, project: &mut ProjectType) {
    if root.join("lerna.json").exists()
        || root.join("nx.json").exists()
        || root.join("turbo.json").exists()
        || root.join("rush.json").exists()
    {
        project.is_monorepo = true;
    }
}
