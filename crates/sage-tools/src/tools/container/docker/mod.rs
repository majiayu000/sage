//! Docker Tool
//!
//! This tool provides Docker container management operations including:
//! - Container lifecycle management
//! - Image building and management
//! - Volume and network operations
//! - Docker Compose integration
//! - Registry operations

pub mod types;
pub mod schema;
pub mod commands;
pub mod tool;

#[cfg(test)]
mod tests;

// Re-export public items
pub use types::{DockerOperation, DockerParams};
pub use tool::DockerTool;
