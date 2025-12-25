//! Jupyter notebook data structures

use serde::{Deserialize, Serialize};

/// Jupyter notebook cell representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub cell_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_count: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub source: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<serde_json::Value>>,
}

/// Jupyter notebook structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub cells: Vec<NotebookCell>,
    pub metadata: serde_json::Value,
    pub nbformat: u32,
    pub nbformat_minor: u32,
}
