//! Core SageBuilder struct and basic construction methods

use crate::agent::lifecycle::LifecycleHook;
use crate::cache::CacheConfig;
use crate::config::model::ModelParameters;
use crate::config::provider::ProviderConfig;
use crate::mcp::transport::TransportConfig;
use crate::tools::base::Tool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Builder for Sage agents
pub struct SageBuilder {
    /// Base configuration
    pub(super) config: Option<crate::config::model::Config>,
    /// Provider configurations
    pub(super) providers: HashMap<String, ProviderConfig>,
    /// Default provider name
    pub(super) default_provider: Option<String>,
    /// Model parameters override
    pub(super) model_params: Option<ModelParameters>,
    /// Tools to register
    pub(super) tools: Vec<Arc<dyn Tool>>,
    /// Lifecycle hooks
    pub(super) hooks: Vec<Arc<dyn LifecycleHook>>,
    /// MCP server configurations
    pub(super) mcp_servers: Vec<(String, TransportConfig)>,
    /// Trajectory output path
    pub(super) trajectory_path: Option<PathBuf>,
    /// Cache configuration
    pub(super) cache_config: Option<CacheConfig>,
    /// Event bus capacity
    pub(super) event_bus_capacity: usize,
    /// Max steps
    pub(super) max_steps: Option<u32>,
    /// Working directory
    pub(super) working_dir: Option<PathBuf>,
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
}

/// Quick builder functions for common configurations
impl SageBuilder {
    /// Create a minimal builder with OpenAI
    pub fn minimal_openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        use crate::builder::config::ConfigBuilderExt;

        Self::new()
            .with_openai(api_key)
            .with_model(model)
            .with_max_steps(20)
    }

    /// Create a minimal builder with Anthropic
    pub fn minimal_anthropic(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        use crate::builder::config::ConfigBuilderExt;

        Self::new()
            .with_anthropic(api_key)
            .with_model(model)
            .with_max_steps(20)
    }

    /// Create a minimal builder with Google
    pub fn minimal_google(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        use crate::builder::config::ConfigBuilderExt;

        Self::new()
            .with_google(api_key)
            .with_model(model)
            .with_max_steps(20)
    }

    /// Create a development builder with logging and metrics
    pub fn development() -> Self {
        use crate::agent::lifecycle::{LoggingHook, MetricsHook};
        use crate::builder::config::ConfigBuilderExt;

        Self::new()
            .with_hook(Arc::new(LoggingHook::all_phases()))
            .with_hook(Arc::new(MetricsHook::new()))
            .with_max_steps(50)
    }

    /// Create a production builder with conservative settings
    pub fn production() -> Self {
        use crate::agent::lifecycle::MetricsHook;
        use crate::builder::config::ConfigBuilderExt;

        Self::new()
            .with_hook(Arc::new(MetricsHook::new()))
            .with_max_steps(100)
            .with_event_bus_capacity(10000)
    }
}
