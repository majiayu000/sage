//! SageBuilder - Fluent builder for constructing Sage agents
//!
//! Provides a convenient builder pattern for creating fully configured agents
//! with all necessary components.

mod build;
mod components;
mod config;
mod core;
mod error;

#[cfg(test)]
mod tests;

// Re-export public API
pub use components::SageComponents;
pub use config::ConfigBuilderExt;
pub use core::SageBuilder;
pub use error::BuilderError;
