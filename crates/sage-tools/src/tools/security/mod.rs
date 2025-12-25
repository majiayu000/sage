//! Security Tools
//!
//! This module provides security-related tools for vulnerability scanning,
//! secret management, and security analysis.

mod scanner;

pub use scanner::{
    SecurityScannerTool,
    ScanType,
    SecurityOperation,
    SecurityScannerParams,
    SecurityFinding,
    ScanResult,
};

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all security tools
pub fn get_security_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(SecurityScannerTool::new()),
    ]
}