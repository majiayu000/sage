//! Container Management Tools
//!
//! This module provides tools for container management including Docker operations.

pub mod docker;

pub use docker::DockerTool;

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all container tools
pub fn get_container_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(DockerTool::new()),
    ]
}