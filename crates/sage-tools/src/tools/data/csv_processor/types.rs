//! CSV processor types and enums
//!
//! This module contains the core types and enums used by the CSV processor.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Data format types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataFormat {
    Csv,
    Excel,
    Tsv,
    Json,
}

/// CSV/Excel operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CsvOperation {
    /// Read data from file
    Read {
        file_path: String,
        format: DataFormat,
        delimiter: Option<String>,
        has_headers: Option<bool>,
        encoding: Option<String>,
    },
    /// Write data to file
    Write {
        file_path: String,
        format: DataFormat,
        data: Vec<HashMap<String, serde_json::Value>>,
        delimiter: Option<String>,
        include_headers: Option<bool>,
    },
    /// Transform data
    Transform {
        input_file: String,
        output_file: String,
        operations: Vec<TransformOperation>,
    },
    /// Analyze data
    Analyze {
        file_path: String,
        columns: Option<Vec<String>>,
    },
    /// Validate data
    Validate {
        file_path: String,
        schema: super::schema::ValidationSchema,
    },
    /// Convert format
    Convert {
        input_file: String,
        output_file: String,
        input_format: DataFormat,
        output_format: DataFormat,
    },
    /// Filter data
    Filter {
        input_file: String,
        output_file: String,
        conditions: Vec<FilterCondition>,
    },
    /// Merge files
    Merge {
        input_files: Vec<String>,
        output_file: String,
        join_type: JoinType,
        join_columns: Vec<String>,
    },
}

/// Data transformation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformOperation {
    /// Add column
    AddColumn {
        name: String,
        value: serde_json::Value,
    },
    /// Remove column
    RemoveColumn {
        name: String,
    },
    /// Rename column
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    /// Apply function to column
    ApplyFunction {
        column: String,
        function: String, // e.g., "upper", "lower", "trim", "substring"
        params: Option<serde_json::Value>,
    },
    /// Sort by column
    Sort {
        column: String,
        ascending: bool,
    },
    /// Group by column
    GroupBy {
        columns: Vec<String>,
        aggregations: HashMap<String, String>, // column -> function (sum, avg, count, etc.)
    },
}

/// Filter conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub column: String,
    pub operator: String, // eq, ne, gt, lt, gte, lte, contains, starts_with, ends_with
    pub value: serde_json::Value,
}

/// Join types for merging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Outer,
}

/// CSV processor parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvProcessorParams {
    /// CSV operation
    pub operation: CsvOperation,
    /// Working directory
    pub working_dir: Option<String>,
}

/// Data analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAnalysis {
    pub total_rows: usize,
    pub total_columns: usize,
    pub column_stats: HashMap<String, ColumnStats>,
    pub missing_values: HashMap<String, usize>,
    pub data_types: HashMap<String, String>,
}

/// Column statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub count: usize,
    pub unique_count: usize,
    pub null_count: usize,
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub std_dev: Option<f64>,
}
