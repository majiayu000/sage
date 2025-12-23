//! Test framework type definitions

use serde::{Deserialize, Serialize};

/// Test framework
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestFramework {
    // Rust
    RustBuiltin,
    // JavaScript/TypeScript
    Jest,
    Vitest,
    Mocha,
    Playwright,
    Cypress,
    // Python
    Pytest,
    Unittest,
    // Go
    GoTest,
    // Java
    JUnit,
    TestNG,
    // Other
    RSpec,
    PHPUnit,
    Custom(String),
}

impl TestFramework {
    /// Get the test framework name
    pub fn name(&self) -> &str {
        match self {
            Self::RustBuiltin => "Rust built-in",
            Self::Jest => "Jest",
            Self::Vitest => "Vitest",
            Self::Mocha => "Mocha",
            Self::Playwright => "Playwright",
            Self::Cypress => "Cypress",
            Self::Pytest => "pytest",
            Self::Unittest => "unittest",
            Self::GoTest => "go test",
            Self::JUnit => "JUnit",
            Self::TestNG => "TestNG",
            Self::RSpec => "RSpec",
            Self::PHPUnit => "PHPUnit",
            Self::Custom(name) => name,
        }
    }
}
