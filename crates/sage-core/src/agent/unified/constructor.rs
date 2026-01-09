//! Constructor logic for UnifiedExecutor

use crate::agent::ExecutionOptions;
use crate::config::model::Config;
use crate::context::{AutoCompact, AutoCompactConfig};
use crate::error::SageResult;
use crate::hooks::{HookExecutor, HookRegistry};
use crate::llm::model_capabilities::get_model_capability;
use crate::skills::SkillRegistry;
use crate::tools::executor::ToolExecutor;
use anyhow::Context;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::event_manager::EventManager;
use super::llm_orchestrator::LlmOrchestrator;
use super::session_manager::SessionManager;
use super::tool_orchestrator::ToolOrchestrator;
use super::{UnifiedExecutor, input_channel};

impl UnifiedExecutor {
    /// Create a new unified executor with default options
    pub fn new(config: Config) -> SageResult<Self> {
        Self::with_options(config, ExecutionOptions::default())
    }

    /// Create a new unified executor with custom options
    pub fn with_options(config: Config, options: ExecutionOptions) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config
            .default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        tracing::info!(
            "Creating unified executor with provider: {}, model: {}",
            provider_name,
            default_params.model
        );

        // Create LLM orchestrator (handles all LLM communication)
        let llm_orchestrator = LlmOrchestrator::from_config(&config)
            .context("Failed to create LLM orchestrator")?;

        // Create tool executor and hook executor
        let tool_executor = ToolExecutor::new();
        let hook_executor = HookExecutor::new(HookRegistry::new());

        // Create tool orchestrator with three-phase execution model
        let tool_orchestrator = ToolOrchestrator::new(tool_executor, hook_executor);

        // Create input channel based on mode
        let input_channel = input_channel::create_input_channel(&options);

        // Create event manager for unified event handling
        let event_manager = EventManager::new();

        // Get working directory for context
        let working_dir = options
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Create skill registry and register built-in skills (before working_dir is moved)
        let mut skill_registry = SkillRegistry::new(&working_dir);

        // Create session manager with working directory
        let session_manager = SessionManager::new(working_dir);

        // Create auto-compact manager with model-specific context window
        let model_capability = get_model_capability(&default_params.model);
        let auto_compact_config =
            AutoCompactConfig::default().with_max_tokens(model_capability.context_window as usize);
        let auto_compact = AutoCompact::new(auto_compact_config);

        tracing::debug!(
            "Auto-compact initialized with max context: {} tokens",
            model_capability.context_window
        );
        skill_registry.register_builtins();
        tracing::debug!(
            "Skill registry initialized with {} built-in skills",
            skill_registry.builtin_count()
        );

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_orchestrator,
            tool_orchestrator,
            options,
            input_channel,
            event_manager,
            session_manager,
            auto_compact,
            skill_registry: Arc::new(RwLock::new(skill_registry)),
        })
    }
}
