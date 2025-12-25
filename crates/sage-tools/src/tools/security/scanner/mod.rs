//! Security Scanner Tool
//!
//! This module provides security scanning capabilities including:
//! - Static Application Security Testing (SAST)
//! - Dependency vulnerability scanning
//! - Secret detection
//! - License compliance checking
//! - Code quality analysis

mod types;
mod schema;
mod patterns;
mod operations;
mod scanner;
mod formatter;
mod tool;

#[cfg(test)]
mod tests;

// Re-export public types
pub use types::{
    ScanType,
    SecurityOperation,
    SecurityScannerParams,
    SecurityFinding,
    ScanResult,
};

pub use tool::SecurityScannerTool;
