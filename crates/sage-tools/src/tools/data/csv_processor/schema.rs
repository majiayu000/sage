//! Validation schema types
//!
//! This module contains types for data validation and schema definition.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Data validation schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchema {
    pub columns: HashMap<String, ColumnSchema>,
}

/// Column validation schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub data_type: String, // string, integer, float, boolean, date
    pub required: bool,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>, // regex pattern
    pub allowed_values: Option<Vec<serde_json::Value>>,
}
