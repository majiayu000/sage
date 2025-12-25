//! CSV processor tool implementation
//!
//! This module contains the Tool trait implementation for CSV processing.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::info;

use sage_core::tools::{Tool, ToolResult};
use super::processor::CsvProcessor;
use super::types::CsvProcessorParams;

/// CSV processor tool
#[derive(Debug, Clone)]
pub struct CsvProcessorTool {
    name: String,
    description: String,
    processor: CsvProcessor,
}

impl CsvProcessorTool {
    /// Create a new CSV processor tool
    pub fn new() -> Self {
        Self {
            name: "csv_processor".to_string(),
            description: "CSV and Excel data processing including reading, writing, transformation, analysis, and validation".to_string(),
            processor: CsvProcessor::new(),
        }
    }
}

impl Default for CsvProcessorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CsvProcessorTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "object",
                    "oneOf": [
                        {
                            "properties": {
                                "read": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": { "type": "string" },
                                        "format": {
                                            "type": "string",
                                            "enum": ["csv", "excel", "tsv", "json"]
                                        },
                                        "delimiter": { "type": "string" },
                                        "has_headers": { "type": "boolean" },
                                        "encoding": { "type": "string" }
                                    },
                                    "required": ["file_path", "format"]
                                }
                            },
                            "required": ["read"]
                        },
                        {
                            "properties": {
                                "analyze": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": { "type": "string" },
                                        "columns": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["file_path"]
                                }
                            },
                            "required": ["analyze"]
                        },
                        {
                            "properties": {
                                "convert": {
                                    "type": "object",
                                    "properties": {
                                        "input_file": { "type": "string" },
                                        "output_file": { "type": "string" },
                                        "input_format": {
                                            "type": "string",
                                            "enum": ["csv", "excel", "tsv", "json"]
                                        },
                                        "output_format": {
                                            "type": "string",
                                            "enum": ["csv", "excel", "tsv", "json"]
                                        }
                                    },
                                    "required": ["input_file", "output_file", "input_format", "output_format"]
                                }
                            },
                            "required": ["convert"]
                        }
                    ]
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for file operations"
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: CsvProcessorParams = serde_json::from_value(params)
            .context("Failed to parse CSV processor parameters")?;

        info!("Executing CSV operation: {:?}", params.operation);

        let result = self.processor.execute_operation(params.operation, params.working_dir.as_deref()).await?;
        let formatted_result = serde_json::to_string_pretty(&result)?;

        let metadata = HashMap::new();

        Ok(ToolResult::new(formatted_result, metadata))
    }
}
