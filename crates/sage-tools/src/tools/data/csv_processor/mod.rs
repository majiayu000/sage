//! CSV/Excel Processor Tool
//!
//! This tool provides CSV and Excel file processing capabilities including:
//! - Data reading and writing
//! - Data transformation and analysis
//! - Data validation and cleaning
//! - Format conversion
//! - Statistical analysis

pub mod schema;
pub mod types;
pub mod processor;
pub mod tool;

#[cfg(test)]
mod tests;

// Re-export public items
pub use schema::{ValidationSchema, ColumnSchema};
pub use types::{
    DataFormat, CsvOperation, TransformOperation, FilterCondition, JoinType,
    CsvProcessorParams, DataAnalysis, ColumnStats,
};
pub use processor::CsvProcessor;
pub use tool::CsvProcessorTool;
