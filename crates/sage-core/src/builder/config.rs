//! Configuration methods for SageBuilder

use super::core::SageBuilder;
use crate::agent::lifecycle::LifecycleHook;
use crate::cache::CacheConfig;
use crate::config::model::{Config, ModelParameters};
use crate::config::provider::ProviderConfig;
use crate::error::SageResult;
use crate::llm::provider_types::TimeoutConfig;
use crate::mcp::transport::TransportConfig;
use crate::tools::base::Tool;
use std::path::PathBuf;
use std::sync::Arc;

/// Extension trait for configuration methods
pub trait ConfigBuilderExt {
    /// Set configuration from a Config object
    fn with_config(self, config: Config) -> Self;

    /// Set configuration from a file path
    fn with_config_file(self, path: impl Into<PathBuf>) -> SageResult<Self>
    where
        Self: Sized;

    /// Add an OpenAI provider
    fn with_openai(self, api_key: impl Into<String>) -> Self;

    /// Add an Anthropic provider
    fn with_anthropic(self, api_key: impl Into<String>) -> Self;

    /// Add a Google provider
    fn with_google(self, api_key: impl Into<String>) -> Self;

    /// Add a custom provider
    fn with_provider(self, name: impl Into<String>, config: ProviderConfig) -> Self;

    /// Set the default provider
    fn with_default_provider(self, provider: impl Into<String>) -> Self;

    /// Set model parameters
    fn with_model(self, model: impl Into<String>) -> Self;

    /// Set temperature
    fn with_temperature(self, temperature: f32) -> Self;

    /// Set max tokens
    fn with_max_tokens(self, max_tokens: u32) -> Self;

    /// Add a tool
    fn with_tool(self, tool: Arc<dyn Tool>) -> Self;

    /// Add multiple tools
    fn with_tools(self, tools: Vec<Arc<dyn Tool>>) -> Self;

    /// Add a lifecycle hook
    fn with_hook(self, hook: Arc<dyn LifecycleHook>) -> Self;

    /// Add multiple lifecycle hooks
    fn with_hooks(self, hooks: Vec<Arc<dyn LifecycleHook>>) -> Self;

    /// Add an MCP server
    fn with_mcp_server(self, name: impl Into<String>, config: TransportConfig) -> Self;

    /// Add an MCP stdio server
    fn with_mcp_stdio_server(
        self,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self;

    /// Set trajectory output path
    fn with_trajectory_path(self, path: impl Into<PathBuf>) -> Self;

    /// Set cache configuration
    fn with_cache_config(self, config: CacheConfig) -> Self;

    /// Enable caching with default configuration
    fn with_cache(self) -> Self;

    /// Set event bus capacity
    fn with_event_bus_capacity(self, capacity: usize) -> Self;

    /// Set max steps
    fn with_max_steps(self, max_steps: u32) -> Self;

    /// Set working directory
    fn with_working_dir(self, path: impl Into<PathBuf>) -> Self;
}

impl ConfigBuilderExt for SageBuilder {
    fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    fn with_config_file(mut self, path: impl Into<PathBuf>) -> SageResult<Self> {
        let config = crate::config::load_config_from_file(path.into())?;
        self.config = Some(config);
        Ok(self)
    }

    fn with_openai(mut self, api_key: impl Into<String>) -> Self {
        let config = ProviderConfig::new("openai")
            .with_api_key(api_key.into())
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3);
        self.providers.insert("openai".to_string(), config);
        if self.default_provider.is_none() {
            self.default_provider = Some("openai".to_string());
        }
        self
    }

    fn with_anthropic(mut self, api_key: impl Into<String>) -> Self {
        let config = ProviderConfig::new("anthropic")
            .with_api_key(api_key.into())
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3);
        self.providers.insert("anthropic".to_string(), config);
        if self.default_provider.is_none() {
            self.default_provider = Some("anthropic".to_string());
        }
        self
    }

    fn with_google(mut self, api_key: impl Into<String>) -> Self {
        let config = ProviderConfig::new("google")
            .with_api_key(api_key.into())
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3);
        self.providers.insert("google".to_string(), config);
        if self.default_provider.is_none() {
            self.default_provider = Some("google".to_string());
        }
        self
    }

    fn with_provider(mut self, name: impl Into<String>, config: ProviderConfig) -> Self {
        let name = name.into();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, config);
        self
    }

    fn with_default_provider(mut self, provider: impl Into<String>) -> Self {
        self.default_provider = Some(provider.into());
        self
    }

    fn with_model(mut self, model: impl Into<String>) -> Self {
        let params = self.model_params.get_or_insert(ModelParameters::default());
        params.model = model.into();
        self
    }

    fn with_temperature(mut self, temperature: f32) -> Self {
        let params = self.model_params.get_or_insert(ModelParameters::default());
        params.temperature = Some(temperature);
        self
    }

    fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        let params = self.model_params.get_or_insert(ModelParameters::default());
        params.max_tokens = Some(max_tokens);
        self
    }

    fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    fn with_hook(mut self, hook: Arc<dyn LifecycleHook>) -> Self {
        self.hooks.push(hook);
        self
    }

    fn with_hooks(mut self, hooks: Vec<Arc<dyn LifecycleHook>>) -> Self {
        self.hooks.extend(hooks);
        self
    }

    fn with_mcp_server(mut self, name: impl Into<String>, config: TransportConfig) -> Self {
        self.mcp_servers.push((name.into(), config));
        self
    }

    fn with_mcp_stdio_server(
        mut self,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        let config = TransportConfig::stdio(command, args);
        self.mcp_servers.push((name.into(), config));
        self
    }

    fn with_trajectory_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.trajectory_path = Some(path.into());
        self
    }

    fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = Some(config);
        self
    }

    fn with_cache(mut self) -> Self {
        self.cache_config = Some(CacheConfig::default());
        self
    }

    fn with_event_bus_capacity(mut self, capacity: usize) -> Self {
        self.event_bus_capacity = capacity;
        self
    }

    fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }
}
