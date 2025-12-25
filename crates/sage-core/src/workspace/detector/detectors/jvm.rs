//! JVM languages project detection (Java, Kotlin, Scala)

use crate::workspace::detector::types::{BuildSystem, LanguageType, ProjectType, RuntimeType};
use std::path::Path;

/// Detects Java/JVM projects via Maven, Gradle, or sbt
pub(super) fn detect(root: &Path, project: &mut ProjectType) {
    detect_maven(root, project);
    detect_gradle(root, project);
    detect_sbt(root, project);
}

fn detect_maven(root: &Path, project: &mut ProjectType) {
    if !root.join("pom.xml").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Java;
    }
    project.build_systems.insert(BuildSystem::Maven);
    project.runtime = Some(RuntimeType::Jvm);
}

fn detect_gradle(root: &Path, project: &mut ProjectType) {
    if !root.join("build.gradle").exists() && !root.join("build.gradle.kts").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Java;
    }
    project.build_systems.insert(BuildSystem::Gradle);
    project.runtime = Some(RuntimeType::Jvm);

    // Check for Kotlin
    if root.join("build.gradle.kts").exists() {
        project.secondary_languages.insert(LanguageType::Kotlin);
    }
}

fn detect_sbt(root: &Path, project: &mut ProjectType) {
    if !root.join("build.sbt").exists() {
        return;
    }

    if project.primary_language == LanguageType::Unknown {
        project.primary_language = LanguageType::Scala;
    }
    project.build_systems.insert(BuildSystem::Sbt);
    project.runtime = Some(RuntimeType::Jvm);
}
