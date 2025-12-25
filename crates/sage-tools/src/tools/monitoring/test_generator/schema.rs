//! Tool schema definition for test generator

use sage_core::tools::types::{ToolParameter, ToolSchema};

use super::types::TestGeneratorTool;

impl TestGeneratorTool {
    /// Get the tool schema
    pub fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "command",
                    "Test generation command (unit_test, integration_test, mock, test_data)",
                ),
                ToolParameter::optional_string(
                    "function_name",
                    "Function name for unit test generation",
                ),
                ToolParameter::optional_string("file_path", "Source file path"),
                ToolParameter::optional_string("module_name", "Module name for integration tests"),
                ToolParameter::optional_string("trait_name", "Trait name for mock generation"),
                ToolParameter::optional_string(
                    "language",
                    "Programming language (rust, typescript, etc.)",
                ),
                ToolParameter::optional_string(
                    "data_type",
                    "Type of test data to generate (user, product, etc.)",
                ),
                ToolParameter::optional_string("format", "Data format (json, csv, etc.)"),
                ToolParameter::optional_string(
                    "test_type",
                    "Type of integration test (api, database, etc.)",
                ),
            ],
        )
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        &self.description
    }
}
