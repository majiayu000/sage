//! Configuration management for Sage Agent

// Internal modules
mod args_loader;
#[allow(clippy::module_inception)] // config module in config directory is intentional
mod config;
mod defaults;
mod env_loader;
mod file_loader;
mod lakeview_config;
mod logging_config;
mod mcp_config;
mod model_params;
mod provider_defaults;
mod tool_config;
mod trajectory_config;

// Public modules
pub mod credential;
pub mod loader;
pub mod model;
pub mod onboarding;
pub mod provider;
pub mod timeouts;
pub mod validation;

// Re-export public API
pub use config::Config;
pub use defaults::{load_config, load_config_from_file, load_config_with_overrides};
pub use lakeview_config::LakeviewConfig;
pub use loader::{ConfigLoader, ConfigSource};
pub use logging_config::LoggingConfig;
pub use mcp_config::{McpConfig, McpServerConfig};
pub use model_params::{ModelParameters, format_api_key_status_for_provider};
pub use provider::{
    ApiAuthConfig, ApiKeyInfo, ApiKeySource, NetworkConfig, ProviderConfig, RateLimitConfig,
    ResilienceConfig, format_api_key_status,
};
pub use tool_config::ToolConfig;
pub use trajectory_config::TrajectoryConfig;
pub use validation::ConfigValidator;
