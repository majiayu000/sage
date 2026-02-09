//! Provider-specific configuration
//!
//! This module provides configuration types for LLM providers, organized into
//! focused structs for better separation of concerns:
//!
//! - [`ApiAuthConfig`]: Authentication settings (API key, organization, project)
//! - [`NetworkConfig`]: Network settings (base URL, headers, timeouts)
//! - [`ResilienceConfig`]: Retry and rate limiting settings
//! - [`ProviderConfig`]: Main configuration that composes the above

mod accessors;
mod api_key;
mod auth;
mod config;
mod defaults;
mod network;
mod resilience;

pub use api_key::{
    ApiKeyInfo, ApiKeySource, format_api_key_status, get_standard_env_vars, mask_api_key,
};
pub use auth::ApiAuthConfig;
pub use config::ProviderConfig;
pub use defaults::ProviderDefaults;
pub use network::NetworkConfig;
pub use resilience::{RateLimitConfig, ResilienceConfig};
