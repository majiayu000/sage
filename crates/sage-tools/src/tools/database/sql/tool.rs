//! SQL Database Tool Implementation

use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, Context};
use tracing::info;

use sage_core::tools::{Tool, ToolResult};
use crate::tools::database::sql::types::DatabaseParams;
use crate::tools::database::sql::schema::parameters_json_schema;
use crate::tools::database::sql::execution::{execute_operation, format_result};

/// Database tool for SQL operations
#[derive(Debug, Clone)]
pub struct DatabaseTool {
    name: String,
    description: String,
}

impl DatabaseTool {
    /// Create a new database tool
    pub fn new() -> Self {
        Self {
            name: "database".to_string(),
            description: "SQL database operations supporting PostgreSQL, MySQL, SQLite, and SQL Server".to_string(),
        }
    }
}

impl Default for DatabaseTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        parameters_json_schema()
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: DatabaseParams = serde_json::from_value(params)
            .context("Failed to parse database parameters")?;

        info!("Executing database operation: {:?}", params.operation);

        let result = execute_operation(params).await?;
        let formatted_result = format_result(&result);

        let mut metadata = HashMap::new();
        metadata.insert("execution_time".to_string(), format!("{}ms", result.execution_time));

        if let Some(rows_affected) = result.rows_affected {
            metadata.insert("rows_affected".to_string(), rows_affected.to_string());
        }

        if let Some(columns) = &result.columns {
            metadata.insert("columns".to_string(), columns.join(", "));
        }

        Ok(ToolResult::new(formatted_result, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_tool_creation() {
        let tool = DatabaseTool::new();
        assert_eq!(tool.name(), "database");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_database_tool_schema() {
        let tool = DatabaseTool::new();
        let schema = tool.parameters_json_schema();

        assert!(schema.is_object());
        assert!(schema["properties"]["config"].is_object());
        assert!(schema["properties"]["operation"].is_object());
    }
}
