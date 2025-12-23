//! Runtime type definitions

use serde::{Deserialize, Serialize};

/// Runtime type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    Node,
    Deno,
    Bun,
    Python,
    Jvm,
    DotNet,
    Native,
    Wasm,
    Browser,
    Custom(String),
}

impl RuntimeType {
    /// Get the runtime name
    pub fn name(&self) -> &str {
        match self {
            Self::Node => "Node.js",
            Self::Deno => "Deno",
            Self::Bun => "Bun",
            Self::Python => "Python",
            Self::Jvm => "JVM",
            Self::DotNet => ".NET",
            Self::Native => "Native",
            Self::Wasm => "WebAssembly",
            Self::Browser => "Browser",
            Self::Custom(name) => name,
        }
    }
}
