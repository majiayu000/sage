//! Email tool tests
//!
//! This module contains test cases for the email tool.

#[cfg(test)]
mod tests {
    use crate::tools::data::email::tool::EmailTool;
    use crate::tools::data::email::sender;

    #[tokio::test]
    async fn test_email_tool_creation() {
        let tool = EmailTool::new();
        assert_eq!(tool.name(), "email");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_email_validation() {
        let result = sender::validate_email("test@example.com").await.unwrap();
        assert!(result["valid"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_email_tool_schema() {
        let tool = EmailTool::new();
        let schema = tool.parameters_json_schema();

        assert!(schema.is_object());
        assert!(schema["properties"]["operation"].is_object());
    }
}
