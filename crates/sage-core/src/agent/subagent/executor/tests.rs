//! Tests for sub-agent executor

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use std::sync::Arc;

    use super::super::executor::SubAgentExecutor;
    use super::super::types::{AgentProgress, SubAgentConfig};
    use crate::agent::subagent::registry::AgentRegistry;
    use crate::agent::subagent::types::{AgentDefinition, AgentType, ToolAccessControl};
    use crate::tools::base::{Tool, ToolError};
    use crate::tools::types::{ToolCall, ToolResult, ToolSchema};

    /// Mock tool for testing
    struct MockTool {
        name: String,
        description: String,
    }

    impl MockTool {
        fn new(name: &str, description: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema {
                name: self.name.clone(),
                description: self.description.clone(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            }
        }

        async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success(
                &call.id,
                &self.name,
                format!("Executed {}", self.name),
            ))
        }
    }

    fn create_test_registry() -> Arc<AgentRegistry> {
        let mut registry = AgentRegistry::new();

        registry.register(AgentDefinition {
            agent_type: AgentType::GeneralPurpose,
            name: "General".to_string(),
            description: "General purpose agent".to_string(),
            available_tools: ToolAccessControl::All,
            model: None,
            system_prompt: "You are a helpful assistant.".to_string(),
        });

        registry.register(AgentDefinition {
            agent_type: AgentType::Explore,
            name: "Explorer".to_string(),
            description: "Code exploration agent".to_string(),
            available_tools: ToolAccessControl::Specific(vec![
                "read".to_string(),
                "glob".to_string(),
            ]),
            model: None,
            system_prompt: "You are a code explorer.".to_string(),
        });

        Arc::new(registry)
    }

    #[test]
    fn test_filter_tools() {
        let registry = create_test_registry();
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("read", "Read files")),
            Arc::new(MockTool::new("write", "Write files")),
            Arc::new(MockTool::new("glob", "Search files")),
        ];

        // Mock LLM client (won't be used in this test)
        use crate::config::provider::ProviderConfig;
        use crate::llm::provider_types::{LlmProvider, ModelParameters};

        let llm_config = ProviderConfig::new("openai")
            .with_base_url("http://localhost")
            .with_api_key("test-api-key");

        let model_params = ModelParameters {
            model: "test-model".to_string(),
            ..Default::default()
        };

        let llm_client = Arc::new(
            crate::llm::client::LlmClient::new(LlmProvider::OpenAI, llm_config, model_params)
                .unwrap(),
        );

        let executor = SubAgentExecutor::new(registry.clone(), llm_client, tools);

        // Test filtering for GeneralPurpose agent (should have all tools)
        let general_def = registry.get(&AgentType::GeneralPurpose).unwrap();
        let filtered = executor.filter_tools(&general_def);
        assert_eq!(filtered.len(), 3);

        // Test filtering for Explore agent (should only have read and glob)
        let explore_def = registry.get(&AgentType::Explore).unwrap();
        let filtered = executor.filter_tools(&explore_def);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|t| t.name() == "read"));
        assert!(filtered.iter().any(|t| t.name() == "glob"));
        assert!(!filtered.iter().any(|t| t.name() == "write"));
    }

    #[test]
    fn test_sub_agent_config() {
        let config = SubAgentConfig::new(AgentType::Explore, "Find all Rust files")
            .with_context("Working in /src directory")
            .with_max_steps(10)
            .with_temperature(0.5);

        assert_eq!(config.agent_type, AgentType::Explore);
        assert_eq!(config.task, "Find all Rust files");
        assert_eq!(
            config.context,
            Some("Working in /src directory".to_string())
        );
        assert_eq!(config.max_steps, Some(10));
        assert_eq!(config.temperature, Some(0.5));
    }

    #[test]
    fn test_agent_progress() {
        let progress = AgentProgress::new(5, 10, "Processing");
        assert_eq!(progress.step, 5);
        assert_eq!(progress.max_steps, 10);
        assert_eq!(progress.percentage, 50);

        let progress = AgentProgress::new(10, 10, "Complete");
        assert_eq!(progress.percentage, 100);

        let progress = AgentProgress::new(1, 3, "Starting");
        assert_eq!(progress.percentage, 33);
    }
}
