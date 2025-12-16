//! Infrastructure Management Tools
//!
//! This module provides tools for infrastructure management including Kubernetes,
//! Terraform, and cloud provider integrations.

pub mod cloud;
pub mod kubernetes;
pub mod terraform;

pub use cloud::CloudTool;
pub use kubernetes::KubernetesTool;
pub use terraform::TerraformTool;

use sage_core::tools::Tool;
use std::sync::Arc;

/// Get all infrastructure tools
pub fn get_infrastructure_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(KubernetesTool::new()),
        Arc::new(TerraformTool::new()),
        Arc::new(CloudTool::new()),
    ]
}
