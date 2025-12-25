//! CSV processor tests
//!
//! This module contains tests for the CSV processor tool.

#[cfg(test)]
mod tests {
    use super::super::tool::CsvProcessorTool;
    use sage_core::tools::Tool;

    #[tokio::test]
    async fn test_csv_processor_tool_creation() {
        let tool = CsvProcessorTool::new();
        assert_eq!(tool.name(), "csv_processor");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_csv_processor_schema() {
        let tool = CsvProcessorTool::new();
        let schema = tool.parameters_json_schema();

        assert!(schema.is_object());
        assert!(schema["properties"]["operation"].is_object());
    }
}
