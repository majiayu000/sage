//! Security scanner tool implementation

use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, Context};
use tracing::info;

use sage_core::tools::{Tool, ToolResult};

use crate::tools::security::scanner::types::SecurityScannerParams;
use crate::tools::security::scanner::schema::get_parameters_schema;
use crate::tools::security::scanner::scanner::execute_scan;
use crate::tools::security::scanner::formatter::format_result;

/// Security scanner tool
#[derive(Debug, Clone)]
pub struct SecurityScannerTool {
    name: String,
    description: String,
}

impl SecurityScannerTool {
    /// Create a new security scanner tool
    pub fn new() -> Self {
        Self {
            name: "security_scanner".to_string(),
            description: "Security vulnerability scanning including SAST, dependency analysis, secret detection, and compliance checking".to_string(),
        }
    }
}

impl Default for SecurityScannerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SecurityScannerTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        get_parameters_schema()
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: SecurityScannerParams = serde_json::from_value(params)
            .context("Failed to parse security scanner parameters")?;

        info!("Executing security scan operation: {:?}", params.operation);

        let result = execute_scan(params.operation, params.working_dir.as_deref()).await?;
        let formatted_result = format_result(&result);

        let mut metadata = HashMap::new();
        metadata.insert("scan_type".to_string(), format!("{:?}", result.scan_type));
        metadata.insert("duration".to_string(), format!("{:.2}s", result.duration));
        metadata.insert("status".to_string(), result.status);

        let total_findings: usize = result.summary.values().sum();
        metadata.insert("total_findings".to_string(), total_findings.to_string());

        Ok(ToolResult::new(formatted_result, metadata))
    }
}
