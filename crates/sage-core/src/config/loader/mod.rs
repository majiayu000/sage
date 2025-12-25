//! Configuration loading and management
//!
//! This module provides the configuration loading system with support for multiple sources:
//! - Configuration files (JSON, TOML, YAML)
//! - Environment variables
//! - Command line arguments
//! - Default configuration
//!
//! The loader supports merging configurations from multiple sources, with later sources
//! overriding earlier ones.

mod builder;
mod loading;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use builder::ConfigLoader;
pub use types::ConfigSource;
