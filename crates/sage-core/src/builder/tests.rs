//! Tests for the builder module

#[cfg(test)]
mod tests {
    use super::super::config::ConfigBuilderExt;
    use super::super::core::SageBuilder;
    use super::super::error::BuilderError;
    use crate::agent::lifecycle::{
        HookResult, LifecycleContext, LifecycleHook, LifecyclePhase, LifecycleResult,
    };
    use crate::tools::base::Tool;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct TestTool {
        name: String,
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "Test tool"
        }

        fn schema(&self) -> crate::tools::types::ToolSchema {
            crate::tools::types::ToolSchema::new(self.name.clone(), "Test tool".to_string(), vec![])
        }

        async fn execute(
            &self,
            _call: &crate::tools::types::ToolCall,
        ) -> Result<crate::tools::types::ToolResult, crate::tools::base::ToolError> {
            Ok(crate::tools::types::ToolResult::success(
                "test-id", &self.name, "success",
            ))
        }
    }

    struct TestHook {
        name: String,
    }

    #[async_trait]
    impl LifecycleHook for TestHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn phases(&self) -> Vec<LifecyclePhase> {
            vec![LifecyclePhase::Init]
        }

        async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
            Ok(HookResult::Continue)
        }
    }

    #[test]
    fn test_builder_new() {
        let builder = SageBuilder::new();
        assert!(builder.config.is_none());
        assert!(builder.tools.is_empty());
        assert!(builder.hooks.is_empty());
    }

    #[test]
    fn test_builder_with_openai() {
        let builder = SageBuilder::new().with_openai("test-key");
        assert!(builder.providers.contains_key("openai"));
        assert_eq!(builder.default_provider, Some("openai".to_string()));
    }

    #[test]
    fn test_builder_with_anthropic() {
        let builder = SageBuilder::new().with_anthropic("test-key");
        assert!(builder.providers.contains_key("anthropic"));
        assert_eq!(builder.default_provider, Some("anthropic".to_string()));
    }

    #[test]
    fn test_builder_with_model() {
        let builder = SageBuilder::new()
            .with_model("gpt-4")
            .with_temperature(0.7)
            .with_max_tokens(4096);

        let params = builder.model_params.unwrap();
        assert_eq!(params.model, "gpt-4");
        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.max_tokens, Some(4096));
    }

    #[test]
    fn test_builder_with_tools() {
        let tool = Arc::new(TestTool {
            name: "test".to_string(),
        });
        let builder = SageBuilder::new().with_tool(tool);
        assert_eq!(builder.tools.len(), 1);
    }

    #[test]
    fn test_builder_with_hooks() {
        let hook = Arc::new(TestHook {
            name: "test".to_string(),
        });
        let builder = SageBuilder::new().with_hook(hook);
        assert_eq!(builder.hooks.len(), 1);
    }

    #[test]
    fn test_builder_with_mcp_server() {
        let builder =
            SageBuilder::new().with_mcp_stdio_server("test", "echo", vec!["hello".to_string()]);
        assert_eq!(builder.mcp_servers.len(), 1);
    }

    #[test]
    fn test_builder_with_working_dir() {
        let builder = SageBuilder::new().with_working_dir("/tmp");
        assert!(builder.working_dir.is_some());
    }

    #[test]
    fn test_builder_with_cache() {
        let builder = SageBuilder::new().with_cache();
        assert!(builder.cache_config.is_some());
    }

    #[test]
    fn test_builder_build_tool_executor() {
        let tool = Arc::new(TestTool {
            name: "test".to_string(),
        });
        let builder = SageBuilder::new().with_tool(tool);
        let executor = builder.build_tool_executor().unwrap();
        assert_eq!(executor.get_tool_schemas().len(), 1);
    }

    #[tokio::test]
    async fn test_builder_build_lifecycle_manager() {
        let hook = Arc::new(TestHook {
            name: "test".to_string(),
        });
        let builder = SageBuilder::new().with_hook(hook);
        let manager = builder.build_lifecycle_manager().await.unwrap();
        assert_eq!(manager.registry().count().await, 1);
    }

    #[test]
    fn test_builder_build_event_bus() {
        let builder = SageBuilder::new().with_event_bus_capacity(500);
        let _bus = builder.build_event_bus();
        // EventBus created successfully
    }

    #[test]
    fn test_builder_build_cancellation() {
        let builder = SageBuilder::new();
        let _cancel = builder.build_cancellation_hierarchy();
        // CancellationHierarchy created successfully
    }

    #[test]
    fn test_builder_build_session_recorder() {
        let builder = SageBuilder::new().with_working_dir("/tmp");
        let recorder = builder.build_session_recorder().unwrap();
        assert!(recorder.is_some());
    }

    #[test]
    fn test_builder_minimal_openai() {
        let builder = SageBuilder::minimal_openai("key", "gpt-4");
        assert!(builder.providers.contains_key("openai"));
        assert_eq!(builder.model_params.unwrap().model, "gpt-4");
    }

    #[test]
    fn test_builder_minimal_anthropic() {
        let builder = SageBuilder::minimal_anthropic("key", "claude-3-opus");
        assert!(builder.providers.contains_key("anthropic"));
        assert_eq!(builder.model_params.unwrap().model, "claude-3-opus");
    }

    #[test]
    fn test_builder_development() {
        let builder = SageBuilder::development();
        assert_eq!(builder.hooks.len(), 2);
        assert_eq!(builder.max_steps, Some(50));
    }

    #[test]
    fn test_builder_production() {
        let builder = SageBuilder::production();
        assert_eq!(builder.hooks.len(), 1);
        assert_eq!(builder.max_steps, Some(100));
        assert_eq!(builder.event_bus_capacity, 10000);
    }

    #[test]
    fn test_builder_error_display() {
        let err = BuilderError::MissingConfig("test".to_string());
        assert!(err.to_string().contains("Missing configuration"));

        let err = BuilderError::InvalidConfig("test".to_string());
        assert!(err.to_string().contains("Invalid configuration"));

        let err = BuilderError::InitFailed("test".to_string());
        assert!(err.to_string().contains("Initialization failed"));

        let err = BuilderError::ProviderNotConfigured("test".to_string());
        assert!(err.to_string().contains("Provider not configured"));
    }

    #[tokio::test]
    async fn test_builder_build_mcp_registry() {
        let builder = SageBuilder::new();
        let registry = builder.build_mcp_registry().await.unwrap();
        assert!(registry.server_names().is_empty());
    }

    #[test]
    fn test_builder_chaining() {
        let builder = SageBuilder::new()
            .with_openai("key")
            .with_model("gpt-4")
            .with_temperature(0.5)
            .with_max_tokens(2048)
            .with_max_steps(30)
            .with_working_dir("/tmp")
            .with_cache()
            .with_event_bus_capacity(2000);

        assert!(builder.providers.contains_key("openai"));
        assert_eq!(builder.model_params.as_ref().unwrap().model, "gpt-4");
        assert_eq!(builder.max_steps, Some(30));
        assert!(builder.working_dir.is_some());
        assert!(builder.cache_config.is_some());
        assert_eq!(builder.event_bus_capacity, 2000);
    }
}
