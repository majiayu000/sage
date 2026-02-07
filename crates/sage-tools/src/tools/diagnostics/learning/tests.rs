//! Tests for learning tools

#[cfg(test)]
mod suite {
    use sage_core::tools::{Tool, ToolCall};
    use serde_json::json;

    use crate::tools::diagnostics::learning::{LearnTool, LearningPatternsTool};

    #[tokio::test]
    async fn test_learn_tool() {
        let tool = LearnTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "Learn".to_string(),
            arguments: json!({
                "pattern_type": "correction",
                "description": "Avoid using grep -r",
                "rule": "Use ripgrep (rg) instead of grep -r for better performance",
                "context": "bash,search"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Pattern learned"));
    }

    #[tokio::test]
    async fn test_learning_patterns_list() {
        let learn_tool = LearnTool::new();
        let patterns_tool = LearningPatternsTool::new();

        // Add a pattern first
        let add_call = ToolCall {
            id: "test-1".to_string(),
            name: "Learn".to_string(),
            arguments: json!({
                "pattern_type": "preference",
                "description": "User prefers 4-space indentation",
                "rule": "Use 4 spaces for indentation, not tabs"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };
        learn_tool.execute(&add_call).await.unwrap();

        // List patterns
        let list_call = ToolCall {
            id: "test-2".to_string(),
            name: "LearningPatterns".to_string(),
            arguments: json!({
                "action": "list"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = patterns_tool.execute(&list_call).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_learning_patterns_stats() {
        let tool = LearningPatternsTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "LearningPatterns".to_string(),
            arguments: json!({
                "action": "stats"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Learning Statistics"));
    }
}
