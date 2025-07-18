//! Data Processing Tools
//!
//! This module provides tools for data processing and manipulation.

pub mod csv_processor;
pub mod email;

pub use csv_processor::CsvProcessorTool;
pub use email::EmailTool;

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all data processing tools
pub fn get_data_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(CsvProcessorTool::new()),
        Arc::new(EmailTool::new()),
    ]
}