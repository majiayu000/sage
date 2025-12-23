//! Build system type definitions

use serde::{Deserialize, Serialize};

/// Build system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Pip,
    Poetry,
    Pdm,
    Uv,
    Maven,
    Gradle,
    Sbt,
    Mix,
    GoModules,
    CMake,
    Make,
    Bazel,
    Meson,
    Custom(String),
}

impl BuildSystem {
    /// Get the build system name
    pub fn name(&self) -> &str {
        match self {
            Self::Cargo => "Cargo",
            Self::Npm => "npm",
            Self::Yarn => "Yarn",
            Self::Pnpm => "pnpm",
            Self::Bun => "Bun",
            Self::Pip => "pip",
            Self::Poetry => "Poetry",
            Self::Pdm => "PDM",
            Self::Uv => "uv",
            Self::Maven => "Maven",
            Self::Gradle => "Gradle",
            Self::Sbt => "sbt",
            Self::Mix => "Mix",
            Self::GoModules => "Go Modules",
            Self::CMake => "CMake",
            Self::Make => "Make",
            Self::Bazel => "Bazel",
            Self::Meson => "Meson",
            Self::Custom(name) => name,
        }
    }

    /// Get the config file name
    pub fn config_file(&self) -> Option<&str> {
        match self {
            Self::Cargo => Some("Cargo.toml"),
            Self::Npm | Self::Yarn | Self::Pnpm | Self::Bun => Some("package.json"),
            Self::Poetry => Some("pyproject.toml"),
            Self::Pdm => Some("pyproject.toml"),
            Self::Uv => Some("pyproject.toml"),
            Self::Maven => Some("pom.xml"),
            Self::Gradle => Some("build.gradle"),
            Self::Sbt => Some("build.sbt"),
            Self::Mix => Some("mix.exs"),
            Self::GoModules => Some("go.mod"),
            Self::CMake => Some("CMakeLists.txt"),
            Self::Make => Some("Makefile"),
            Self::Bazel => Some("BUILD"),
            Self::Meson => Some("meson.build"),
            _ => None,
        }
    }
}
