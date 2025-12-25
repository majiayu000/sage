//! Tests for test generator tool

#[cfg(test)]
mod tests {
    use super::super::TestGeneratorTool;
    use sage_core::tools::base::Tool;

    #[tokio::test]
    async fn test_test_generator_creation() {
        let tool = TestGeneratorTool::new();
        assert_eq!(tool.name(), "test_generator");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_test_generator_schema() {
        let tool = TestGeneratorTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "test_generator");
        assert!(!schema.description.is_empty());
    }
}
