//! Framework type definitions

use serde::{Deserialize, Serialize};

/// Framework type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkType {
    // Web frameworks
    React,
    Vue,
    Angular,
    Svelte,
    NextJs,
    Nuxt,
    Express,
    Fastify,
    NestJs,
    Django,
    Flask,
    FastApi,
    Rails,
    Spring,
    Actix,
    Axum,
    Rocket,
    Gin,
    Echo,
    // Mobile
    ReactNative,
    Flutter,
    SwiftUI,
    Jetpack,
    // Other
    Electron,
    Tauri,
    Custom(String),
}

impl FrameworkType {
    /// Get the framework name
    pub fn name(&self) -> &str {
        match self {
            Self::React => "React",
            Self::Vue => "Vue",
            Self::Angular => "Angular",
            Self::Svelte => "Svelte",
            Self::NextJs => "Next.js",
            Self::Nuxt => "Nuxt",
            Self::Express => "Express",
            Self::Fastify => "Fastify",
            Self::NestJs => "NestJS",
            Self::Django => "Django",
            Self::Flask => "Flask",
            Self::FastApi => "FastAPI",
            Self::Rails => "Rails",
            Self::Spring => "Spring",
            Self::Actix => "Actix",
            Self::Axum => "Axum",
            Self::Rocket => "Rocket",
            Self::Gin => "Gin",
            Self::Echo => "Echo",
            Self::ReactNative => "React Native",
            Self::Flutter => "Flutter",
            Self::SwiftUI => "SwiftUI",
            Self::Jetpack => "Jetpack Compose",
            Self::Electron => "Electron",
            Self::Tauri => "Tauri",
            Self::Custom(name) => name,
        }
    }
}
