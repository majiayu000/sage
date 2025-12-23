//! Go project detection

use crate::workspace::detector::types::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, RuntimeType, TestFramework,
};
use std::path::Path;

/// Detects Go projects via go.mod
pub(super) fn detect(root: &Path, project: &mut ProjectType) {
    let go_mod = root.join("go.mod");
    if !go_mod.exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Go;
    }
    project.build_systems.insert(BuildSystem::GoModules);
    project.test_frameworks.insert(TestFramework::GoTest);
    project.runtime = Some(RuntimeType::Native);

    // Detect frameworks from go.mod
    if let Ok(content) = std::fs::read_to_string(&go_mod) {
        if content.contains("gin-gonic/gin") {
            project.frameworks.insert(FrameworkType::Gin);
        }
        if content.contains("labstack/echo") {
            project.frameworks.insert(FrameworkType::Echo);
        }
    }
}
