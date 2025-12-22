//! SageBuilder - Fluent builder for constructing Sage agents
//!
//! Provides a convenient builder pattern for creating fully configured agents
//! with all necessary components.

use crate::agent::ClaudeStyleAgent;
use crate::agent::lifecycle::{LifecycleHook, LifecycleHookRegistry, LifecycleManager};
use crate::cache::CacheConfig;
use crate::concurrency::CancellationHierarchy;
use crate::config::model::{Config, ModelParameters};
use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::events::EventBus;
use crate::llm::client::LLMClient;
use crate::llm::provider_types::LLMProvider;
use crate::mcp::McpRegistry;
use crate::mcp::transport::TransportConfig;
use crate::tools::base::Tool;
use crate::tools::batch_executor::BatchToolExecutor;
use crate::tools::executor::ToolExecutor;
use crate::trajectory::recorder::TrajectoryRecorder;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Builder error types
#[derive(Debug, Clone)]
pub enum BuilderError {
    /// Missing required configuration
    MissingConfig(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Initialization failed
    InitFailed(String),
    /// Provider not configured
    ProviderNotConfigured(String),
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingConfig(msg) => write!(f, "Missing configuration: {}", msg),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::InitFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::ProviderNotConfigured(msg) => write!(f, "Provider not configured: {}", msg),
        }
    }
}

impl std::error::Error for BuilderError {}

impl From<BuilderError> for SageError {
    fn from(err: BuilderError) -> Self {
        SageError::config(err.to_string())
    }
}

/// Builder for Sage agents
pub struct SageBuilder {
    /// Base configuration
    config: Option<Config>,
    /// Provider configurations
    providers: HashMap<String, ProviderConfig>,
    /// Default provider name
    default_provider: Option<String>,
    /// Model parameters override
    model_params: Option<ModelParameters>,
    /// Tools to register
    tools: Vec<Arc<dyn Tool>>,
    /// Lifecycle hooks
    hooks: Vec<Arc<dyn LifecycleHook>>,
    /// MCP server configurations
    mcp_servers: Vec<(String, TransportConfig)>,
    /// Trajectory output path
    trajectory_path: Option<PathBuf>,
    /// Cache configuration
    cache_config: Option<CacheConfig>,
    /// Event bus capacity
    event_bus_capacity: usize,
    /// Max steps
    max_steps: Option<u32>,
    /// Working directory
    working_dir: Option<PathBuf>,
}

impl Default for SageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SageBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            providers: HashMap::new(),
            default_provider: None,
            model_params: None,
            tools: Vec::new(),
            hooks: Vec::new(),
            mcp_servers: Vec::new(),
            trajectory_path: None,
            cache_config: None,
            event_bus_capacity: 1000,
            max_steps: None,
            working_dir: None,
        }
    }

    /// Set configuration from a Config object
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Set configuration from a file path
    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> SageResult<Self> {
        let config = crate::config::loader::load_config_from_file(path.into())?;
        self.config = Some(config);
        Ok(self)
    }

    /// Add an OpenAI provider
    pub fn with_openai(mut self, api_key: impl Into<String>) -> Self {
        let config = ProviderConfig::new("openai")
            .with_api_key(api_key.into())
            .with_timeout(60)
            .with_max_retries(3);
        self.providers.insert("openai".to_string(), config);
        if self.default_provider.is_none() {
            self.default_provider = Some("openai".to_string());
        }
        self
    }

    /// Add an Anthropic provider
    pub fn with_anthropic(mut self, api_key: impl Into<String>) -> Self {
        let config = ProviderConfig::new("anthropic")
            .with_api_key(api_key.into())
            .with_timeout(60)
            .with_max_retries(3);
        self.providers.insert("anthropic".to_string(), config);
        if self.default_provider.is_none() {
            self.default_provider = Some("anthropic".to_string());
        }
        self
    }

    /// Add a Google provider
    pub fn with_google(mut self, api_key: impl Into<String>) -> Self {
        let config = ProviderConfig::new("google")
            .with_api_key(api_key.into())
            .with_timeout(60)
            .with_max_retries(3);
        self.providers.insert("google".to_string(), config);
        if self.default_provider.is_none() {
            self.default_provider = Some("google".to_string());
        }
        self
    }

    /// Add a custom provider
    pub fn with_provider(mut self, name: impl Into<String>, config: ProviderConfig) -> Self {
        let name = name.into();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, config);
        self
    }

    /// Set the default provider
    pub fn with_default_provider(mut self, provider: impl Into<String>) -> Self {
        self.default_provider = Some(provider.into());
        self
    }

    /// Set model parameters
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        let params = self.model_params.get_or_insert(ModelParameters::default());
        params.model = model.into();
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        let params = self.model_params.get_or_insert(ModelParameters::default());
        params.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        let params = self.model_params.get_or_insert(ModelParameters::default());
        params.max_tokens = Some(max_tokens);
        self
    }

    /// Add a tool
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Add a lifecycle hook
    pub fn with_hook(mut self, hook: Arc<dyn LifecycleHook>) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Add multiple lifecycle hooks
    pub fn with_hooks(mut self, hooks: Vec<Arc<dyn LifecycleHook>>) -> Self {
        self.hooks.extend(hooks);
        self
    }

    /// Add an MCP server
    pub fn with_mcp_server(mut self, name: impl Into<String>, config: TransportConfig) -> Self {
        self.mcp_servers.push((name.into(), config));
        self
    }

    /// Add an MCP stdio server
    pub fn with_mcp_stdio_server(
        mut self,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        let config = TransportConfig::stdio(command, args);
        self.mcp_servers.push((name.into(), config));
        self
    }

    /// Set trajectory output path
    pub fn with_trajectory_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.trajectory_path = Some(path.into());
        self
    }

    /// Set cache configuration
    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = Some(config);
        self
    }

    /// Enable caching with default configuration
    pub fn with_cache(mut self) -> Self {
        self.cache_config = Some(CacheConfig::default());
        self
    }

    /// Set event bus capacity
    pub fn with_event_bus_capacity(mut self, capacity: usize) -> Self {
        self.event_bus_capacity = capacity;
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }

    /// Build a ToolExecutor
    pub fn build_tool_executor(&self) -> SageResult<ToolExecutor> {
        let mut executor = ToolExecutor::new();
        for tool in &self.tools {
            executor.register_tool(tool.clone());
        }
        Ok(executor)
    }

    /// Build a BatchToolExecutor
    pub fn build_batch_executor(&self) -> SageResult<BatchToolExecutor> {
        let mut executor = BatchToolExecutor::new();
        for tool in &self.tools {
            executor.register_tool(tool.clone());
        }
        Ok(executor)
    }

    /// Build a LifecycleManager
    pub async fn build_lifecycle_manager(&self) -> SageResult<LifecycleManager> {
        let registry = Arc::new(LifecycleHookRegistry::new());
        for hook in &self.hooks {
            registry.register(hook.clone()).await;
        }
        Ok(LifecycleManager::with_registry(registry))
    }

    /// Build an EventBus
    pub fn build_event_bus(&self) -> EventBus {
        EventBus::new(self.event_bus_capacity)
    }

    /// Build a CancellationHierarchy
    pub fn build_cancellation_hierarchy(&self) -> CancellationHierarchy {
        CancellationHierarchy::new()
    }

    /// Build a TrajectoryRecorder
    pub fn build_trajectory_recorder(&self) -> SageResult<Option<Arc<Mutex<TrajectoryRecorder>>>> {
        if let Some(path) = &self.trajectory_path {
            let recorder = TrajectoryRecorder::new(path.clone())?;
            Ok(Some(Arc::new(Mutex::new(recorder))))
        } else {
            Ok(None)
        }
    }

    /// Build an LLMClient
    pub fn build_llm_client(&self) -> SageResult<LLMClient> {
        // Get provider configuration
        let provider_name = self
            .default_provider
            .clone()
            .or_else(|| {
                self.config
                    .as_ref()
                    .map(|c| c.get_default_provider().to_string())
            })
            .ok_or_else(|| BuilderError::MissingConfig("No provider configured".into()))?;

        let provider_config = self.providers.get(&provider_name).cloned().or_else(|| {
            self.config.as_ref().and_then(|c| {
                c.default_model_parameters().ok().map(|params| {
                    let mut config = ProviderConfig::new(&provider_name)
                        .with_api_key(params.get_api_key().unwrap_or_default())
                        .with_timeout(60)
                        .with_max_retries(3);
                    // Apply custom base_url if configured (for OpenRouter, etc.)
                    if let Some(base_url) = &params.base_url {
                        config = config.with_base_url(base_url.clone());
                    }
                    config
                })
            })
        });

        let provider_config = provider_config
            .ok_or_else(|| BuilderError::ProviderNotConfigured(provider_name.clone()))?;

        // Parse provider
        let provider: LLMProvider = provider_name.parse().map_err(|_| {
            BuilderError::InvalidConfig(format!("Invalid provider: {}", provider_name))
        })?;

        // Get model parameters
        let model_params = if let Some(params) = &self.model_params {
            params.to_llm_parameters()
        } else if let Some(config) = &self.config {
            config.default_model_parameters()?.to_llm_parameters()
        } else {
            ModelParameters::default().to_llm_parameters()
        };

        LLMClient::new(provider, provider_config, model_params)
    }

    /// Build an McpRegistry
    pub async fn build_mcp_registry(&self) -> SageResult<McpRegistry> {
        let registry = McpRegistry::new();
        for (name, config) in &self.mcp_servers {
            registry.register_server(name, config.clone()).await?;
        }
        Ok(registry)
    }

    /// Build a ClaudeStyleAgent
    pub fn build_claude_style_agent(&self) -> SageResult<ClaudeStyleAgent> {
        let config = self
            .config
            .clone()
            .ok_or_else(|| BuilderError::MissingConfig("Configuration required".into()))?;

        ClaudeStyleAgent::new(config)
    }

    /// Build all components into a SageComponents struct
    pub async fn build(&self) -> SageResult<SageComponents> {
        let tool_executor = self.build_tool_executor()?;
        let batch_executor = self.build_batch_executor()?;
        let lifecycle_manager = self.build_lifecycle_manager().await?;
        let event_bus = self.build_event_bus();
        let cancellation = self.build_cancellation_hierarchy();
        let trajectory_recorder = self.build_trajectory_recorder()?;
        let mcp_registry = self.build_mcp_registry().await?;

        Ok(SageComponents {
            tool_executor,
            batch_executor,
            lifecycle_manager,
            event_bus,
            cancellation,
            trajectory_recorder,
            mcp_registry,
            config: self.config.clone(),
            max_steps: self.max_steps.unwrap_or(20),
            working_dir: self.working_dir.clone(),
        })
    }
}

/// All components built by SageBuilder
pub struct SageComponents {
    /// Tool executor for sequential tool execution
    pub tool_executor: ToolExecutor,
    /// Batch executor for parallel tool execution
    pub batch_executor: BatchToolExecutor,
    /// Lifecycle manager with registered hooks
    pub lifecycle_manager: LifecycleManager,
    /// Event bus for pub/sub events
    pub event_bus: EventBus,
    /// Cancellation hierarchy for graceful shutdown
    pub cancellation: CancellationHierarchy,
    /// Optional trajectory recorder
    pub trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    /// MCP registry for external tool servers
    pub mcp_registry: McpRegistry,
    /// Configuration
    pub config: Option<Config>,
    /// Max steps for agent execution
    pub max_steps: u32,
    /// Working directory
    pub working_dir: Option<PathBuf>,
}

impl SageComponents {
    /// Get a shared event bus
    pub fn shared_event_bus(&self) -> Arc<EventBus> {
        Arc::new(EventBus::new(1000))
    }

    /// Get lifecycle manager registry for adding more hooks
    pub fn lifecycle_registry(&self) -> Arc<LifecycleHookRegistry> {
        self.lifecycle_manager.registry()
    }

    /// Initialize the lifecycle manager
    pub async fn initialize(&self) -> SageResult<()> {
        let agent_id = uuid::Uuid::new_v4();
        self.lifecycle_manager.initialize(agent_id).await?;
        Ok(())
    }

    /// Shutdown all components
    pub async fn shutdown(&self) -> SageResult<()> {
        let agent_id = uuid::Uuid::new_v4();
        self.lifecycle_manager.shutdown(agent_id).await?;
        self.mcp_registry.close_all().await?;
        Ok(())
    }
}

/// Quick builder functions for common configurations
impl SageBuilder {
    /// Create a minimal builder with OpenAI
    pub fn minimal_openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new()
            .with_openai(api_key)
            .with_model(model)
            .with_max_steps(20)
    }

    /// Create a minimal builder with Anthropic
    pub fn minimal_anthropic(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new()
            .with_anthropic(api_key)
            .with_model(model)
            .with_max_steps(20)
    }

    /// Create a minimal builder with Google
    pub fn minimal_google(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new()
            .with_google(api_key)
            .with_model(model)
            .with_max_steps(20)
    }

    /// Create a development builder with logging and metrics
    pub fn development() -> Self {
        use crate::agent::lifecycle::{LoggingHook, MetricsHook};

        Self::new()
            .with_hook(Arc::new(LoggingHook::all_phases()))
            .with_hook(Arc::new(MetricsHook::new()))
            .with_max_steps(50)
    }

    /// Create a production builder with conservative settings
    pub fn production() -> Self {
        use crate::agent::lifecycle::MetricsHook;

        Self::new()
            .with_hook(Arc::new(MetricsHook::new()))
            .with_max_steps(100)
            .with_event_bus_capacity(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::lifecycle::{HookResult, LifecycleContext, LifecyclePhase, LifecycleResult};
    use async_trait::async_trait;

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
    fn test_builder_with_trajectory() {
        let builder = SageBuilder::new().with_trajectory_path("/tmp/test.json");
        assert!(builder.trajectory_path.is_some());
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

    #[test]
    fn test_builder_build_batch_executor() {
        let tool = Arc::new(TestTool {
            name: "test".to_string(),
        });
        let builder = SageBuilder::new().with_tool(tool);
        let executor = builder.build_batch_executor().unwrap();
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
    fn test_builder_build_trajectory() {
        let builder = SageBuilder::new().with_trajectory_path("/tmp/test.json");
        let recorder = builder.build_trajectory_recorder().unwrap();
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
