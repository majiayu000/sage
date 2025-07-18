//! Infrastructure Management Tools
//!
//! This module provides tools for infrastructure management including Kubernetes,
//! Terraform, and cloud provider integrations.

pub mod kubernetes;
pub mod terraform;
pub mod cloud;

pub use kubernetes::KubernetesTool;
pub use terraform::TerraformTool;
pub use cloud::CloudTool;

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all infrastructure tools
pub fn get_infrastructure_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(KubernetesTool::new()),
        Arc::new(TerraformTool::new()),
        Arc::new(CloudTool::new()),
    ]
}