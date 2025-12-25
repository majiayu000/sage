//! Build and finalization logic for SageBuilder

use super::components::SageComponents;
use super::core::SageBuilder;
use super::error::BuilderError;
use crate::agent::ClaudeStyleAgent;
use crate::agent::lifecycle::{LifecycleHookRegistry, LifecycleManager};
use crate::concurrency::CancellationHierarchy;
use crate::config::model::ModelParameters;
use crate::error::SageResult;
use crate::events::EventBus;
use crate::llm::client::LlmClient;
use crate::llm::provider_types::LlmProvider;
use crate::mcp::McpRegistry;
use crate::tools::batch_executor::BatchToolExecutor;
use crate::tools::executor::ToolExecutor;
use crate::trajectory::SessionRecorder;
use std::sync::Arc;
use tokio::sync::Mutex;

impl SageBuilder {
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

    /// Build a SessionRecorder
    pub fn build_session_recorder(&self) -> SageResult<Option<Arc<Mutex<SessionRecorder>>>> {
        // SessionRecorder uses working directory, not trajectory path
        // It stores in ~/.sage/projects/{escaped-cwd}/
        if let Some(working_dir) = &self.working_dir {
            let recorder = SessionRecorder::new(working_dir)?;
            Ok(Some(Arc::new(Mutex::new(recorder))))
        } else {
            // Use current directory as fallback
            let recorder = SessionRecorder::new(std::env::current_dir()?)?;
            Ok(Some(Arc::new(Mutex::new(recorder))))
        }
    }

    /// Build an LlmClient
    pub fn build_llm_client(&self) -> SageResult<LlmClient> {
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
                    let mut config = crate::config::provider::ProviderConfig::new(&provider_name)
                        .with_api_key(params.get_api_key().unwrap_or_default())
                        .with_timeouts(crate::llm::provider_types::TimeoutConfig::default())
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
        let provider: LlmProvider = provider_name.parse().map_err(|_| {
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

        LlmClient::new(provider, provider_config, model_params)
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
        let session_recorder = self.build_session_recorder()?;
        let mcp_registry = self.build_mcp_registry().await?;

        Ok(SageComponents {
            tool_executor,
            batch_executor,
            lifecycle_manager,
            event_bus,
            cancellation,
            session_recorder,
            mcp_registry,
            config: self.config.clone(),
            max_steps: self.max_steps.unwrap_or(u32::MAX), // No limit by default
            working_dir: self.working_dir.clone(),
        })
    }
}
