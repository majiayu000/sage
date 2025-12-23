//! Framework detection logic
//!
//! Detects frameworks and test frameworks by analyzing package.json
//! and other configuration files.

use super::types::{FrameworkType, ProjectType, TestFramework};

/// Detects Node.js frameworks from package.json content
pub(super) fn detect_node_frameworks(content: &str, project: &mut ProjectType) {
    let checks = [
        ("react", FrameworkType::React),
        ("vue", FrameworkType::Vue),
        ("@angular/core", FrameworkType::Angular),
        ("svelte", FrameworkType::Svelte),
        ("next", FrameworkType::NextJs),
        ("nuxt", FrameworkType::Nuxt),
        ("express", FrameworkType::Express),
        ("fastify", FrameworkType::Fastify),
        ("@nestjs/core", FrameworkType::NestJs),
        ("electron", FrameworkType::Electron),
        ("react-native", FrameworkType::ReactNative),
    ];

    for (marker, framework) in checks {
        if content.contains(&format!("\"{}\"", marker)) {
            project.frameworks.insert(framework);
        }
    }
}

/// Detects Node.js test frameworks from package.json content
pub(super) fn detect_node_test_frameworks(content: &str, project: &mut ProjectType) {
    if content.contains("\"jest\"") {
        project.test_frameworks.insert(TestFramework::Jest);
    }
    if content.contains("\"vitest\"") {
        project.test_frameworks.insert(TestFramework::Vitest);
    }
    if content.contains("\"mocha\"") {
        project.test_frameworks.insert(TestFramework::Mocha);
    }
    if content.contains("\"playwright\"") || content.contains("\"@playwright/test\"") {
        project.test_frameworks.insert(TestFramework::Playwright);
    }
    if content.contains("\"cypress\"") {
        project.test_frameworks.insert(TestFramework::Cypress);
    }
}
