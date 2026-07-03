//! Configuration management for Sage Agent

// Internal modules
mod api_key_helpers;
mod args_loader;
#[allow(clippy::module_inception)] // config module in config directory is intentional
mod config;
mod data_dir;
mod defaults;
mod embedded_providers;
mod env_loader;
mod file_loader;
mod lakeview_config;
mod logging_config;
mod mcp_config;
mod memory_config;
mod model_catalog;
mod model_params;
pub mod models_api;
mod provider_defaults;
mod tool_config;
mod trajectory_config;

// Public modules
pub mod credential;
pub mod loader;
pub mod model;
pub mod onboarding;
pub mod persistence;
pub mod provider;
pub mod provider_registry;
pub mod timeouts;
pub mod validation;

// Re-export public API
pub use api_key_helpers::format_api_key_status_for_provider;
pub use config::Config;
pub use data_dir::{SAGE_STATE_SUBDIR, default_data_dir, default_data_dir_or_warn};
pub use defaults::{load_config, load_config_from_file, load_config_with_overrides};
pub use lakeview_config::LakeviewConfig;
pub use loader::{ConfigLoader, ConfigSource};
pub use logging_config::LoggingConfig;
pub use mcp_config::{McpAuthConfig, McpAuthKind, McpConfig, McpServerConfig};
pub use memory_config::AgentMemoryConfig;
pub use model_catalog::{
    CatalogCacheEntry, CatalogFreshness, CatalogSource, ModelCatalogManager,
    ProviderCatalogSnapshot, merge_provider_catalog,
};
pub use model_params::ModelParameters;
pub use models_api::{FetchedModel, ModelsApiClient};
pub use persistence::{ConfigPersistence, ConfigUpdate};
pub use provider::{
    ApiAuthConfig, ApiKeyInfo, ApiKeySource, NetworkConfig, ProviderConfig, RateLimitConfig,
    ResilienceConfig, format_api_key_status,
};
pub use provider_registry::{ModelInfo, ProviderInfo, ProviderRegistry};
pub use tool_config::ToolConfig;
pub use trajectory_config::TrajectoryConfig;
pub use validation::ConfigValidator;
