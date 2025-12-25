//! Builder error types

use crate::error::SageError;

/// Builder error types
#[derive(Debug, Clone)]
pub enum BuilderError {
    /// Missing required configuration
    MissingConfig(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Initialization failed
    InitFailed(String),
    /// Provider not configured
    ProviderNotConfigured(String),
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingConfig(msg) => write!(f, "Missing configuration: {}", msg),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::InitFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::ProviderNotConfigured(msg) => write!(f, "Provider not configured: {}", msg),
        }
    }
}

impl std::error::Error for BuilderError {}

impl From<BuilderError> for SageError {
    fn from(err: BuilderError) -> Self {
        SageError::config(err.to_string())
    }
}
