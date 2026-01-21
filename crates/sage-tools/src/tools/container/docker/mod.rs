//! Docker Tool
//!
//! This tool provides Docker container management operations including:
//! - Container lifecycle management
//! - Image building and management
//! - Volume and network operations
//! - Docker Compose integration
//! - Registry operations

pub mod commands;
pub mod schema;
pub mod tool;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export public items
pub use tool::DockerTool;
pub use types::{DockerOperation, DockerParams};
