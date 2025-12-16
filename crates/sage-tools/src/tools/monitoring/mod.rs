//! Monitoring and Analysis Tools
//!
//! This module provides tools for log analysis, system monitoring, and performance analysis.

pub mod log_analyzer;
pub mod test_generator;

pub use log_analyzer::LogAnalyzerTool;
pub use test_generator::TestGeneratorTool;

use sage_core::tools::Tool;
use std::sync::Arc;

/// Get all monitoring tools
pub fn get_monitoring_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(LogAnalyzerTool::new()),
        Arc::new(TestGeneratorTool::new()),
    ]
}
