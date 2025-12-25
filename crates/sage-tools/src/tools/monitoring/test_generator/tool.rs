//! Tool trait implementation for test generator

use async_trait::async_trait;
use tracing::info;

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};

use super::types::TestGeneratorTool;

#[async_trait]
impl Tool for TestGeneratorTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        self.schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        info!("Executing test generation command: {}", command);

        let result = match command.as_str() {
            "unit_test" => {
                let function_name = call.get_string("function_name").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'function_name' parameter".to_string())
                })?;
                let file_path = call.get_string("file_path").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
                })?;
                self.generate_rust_unit_test(&function_name, &file_path)
                    .await?
            }
            "integration_test" => {
                let module_name = call.get_string("module_name").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'module_name' parameter".to_string())
                })?;
                let test_type = call
                    .get_string("test_type")
                    .unwrap_or_else(|| "general".to_string());
                self.generate_integration_test(&module_name, &test_type)
                    .await?
            }
            "mock" => {
                let trait_name = call.get_string("trait_name").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'trait_name' parameter".to_string())
                })?;
                let language = call
                    .get_string("language")
                    .unwrap_or_else(|| "rust".to_string());
                self.generate_mock(&trait_name, &language).await?
            }
            "test_data" => {
                let data_type = call
                    .get_string("data_type")
                    .unwrap_or_else(|| "user".to_string());
                let format = call
                    .get_string("format")
                    .unwrap_or_else(|| "json".to_string());
                self.generate_test_data(&data_type, &format).await?
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}",
                    command
                )));
            }
        };

        Ok(ToolResult::success(call.id.clone(), self.name(), result))
    }
}
